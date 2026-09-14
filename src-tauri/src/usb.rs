use rusb::{Context, DeviceHandle, Direction, TransferType, UsbContext};
use serde::Serialize;
use std::time::Duration;

pub const SONY_VID: u16 = 0x054C;
pub const PS3MCA_PID: u16 = 0x02EA;
const TIMEOUT: Duration = Duration::from_millis(5000);
const READ_CMD_LEN: usize = 144;
const FRAME_SIZE: usize = 128;
pub const FRAME_COUNT: u16 = 1024;
const MAX_CONSECUTIVE_FAILURES: u32 = 8;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdaptorIdentity {
    pub vid: u16,
    pub pid: u16,
    pub bcd_device: String,
    pub manufacturer: String,
    pub product: String,
    pub bus: u8,
    pub address: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareStatus {
    pub state: String,
    pub message: String,
    pub identity: Option<AdaptorIdentity>,
    pub frame: u16,
    pub total: u16,
}

impl HardwareStatus {
    pub fn searching() -> Self {
        Self {
            state: "searching".into(),
            message: "No 054C:02EA yet".into(),
            identity: None,
            frame: 0,
            total: FRAME_COUNT,
        }
    }
}

pub fn probe(ctx: &Context) -> HardwareStatus {
    match find_identity(ctx) {
        Ok(Some(identity)) => HardwareStatus {
            state: "adaptor".into(),
            message: format!(
                "Adaptor {:04X}:{:04X} {}",
                identity.vid,
                identity.pid,
                if identity.product.is_empty() {
                    "(no product string)".to_string()
                } else {
                    identity.product.clone()
                }
            ),
            identity: Some(identity),
            frame: 0,
            total: FRAME_COUNT,
        },
        Ok(None) => HardwareStatus::searching(),
        Err(err) => HardwareStatus {
            state: "error".into(),
            message: err,
            identity: None,
            frame: 0,
            total: FRAME_COUNT,
        },
    }
}

pub fn find_identity(ctx: &Context) -> Result<Option<AdaptorIdentity>, String> {
    for device in ctx.devices().map_err(usb_err)?.iter() {
        let desc = device.device_descriptor().map_err(usb_err)?;
        if desc.vendor_id() != SONY_VID || desc.product_id() != PS3MCA_PID {
            continue;
        }
        let handle = device.open().ok();
        let langs = handle
            .as_ref()
            .and_then(|h| h.read_languages(TIMEOUT).ok())
            .unwrap_or_default();
        let lang = langs.first().copied();
        let manufacturer = match (handle.as_ref(), lang, desc.manufacturer_string_index()) {
            (Some(h), Some(l), Some(_)) => h
                .read_manufacturer_string(l, &desc, TIMEOUT)
                .unwrap_or_default(),
            _ => String::new(),
        };
        let product = match (handle.as_ref(), lang, desc.product_string_index()) {
            (Some(h), Some(l), Some(_)) => h.read_product_string(l, &desc, TIMEOUT).unwrap_or_default(),
            _ => String::new(),
        };
        return Ok(Some(identity_from(&device, &desc, manufacturer, product)));
    }
    Ok(None)
}

pub fn read_card<F>(ctx: &Context, mut on_progress: F) -> Result<(Vec<u8>, AdaptorIdentity), String>
where
    F: FnMut(u16),
{
    let mut adaptor = Adaptor::open(ctx)?;
    adaptor.detect_ps1_card()?;
    let identity = adaptor.identity.clone();
    let mut card = vec![0u8; crate::card::CARD_SIZE];
    let mut failures = 0u32;
    let mut i: u16 = 0;
    while i < FRAME_COUNT {
        match adaptor.read_frame(i) {
            Ok(frame) => {
                let start = i as usize * FRAME_SIZE;
                card[start..start + FRAME_SIZE].copy_from_slice(&frame);
                on_progress(i);
                i += 1;
                failures = 0;
            }
            Err(_) => {
                failures += 1;
                if failures >= MAX_CONSECUTIVE_FAILURES {
                    return Err(format!(
                        "Read failed at frame {i} after {MAX_CONSECUTIVE_FAILURES} consecutive errors."
                    ));
                }
            }
        }
    }
    drop(adaptor);
    Ok((card, identity))
}

struct Adaptor {
    handle: DeviceHandle<Context>,
    in_ep: u8,
    out_ep: u8,
    identity: AdaptorIdentity,
}

impl Adaptor {
    fn open(ctx: &Context) -> Result<Self, String> {
        let mut found = None;
        for device in ctx.devices().map_err(usb_err)?.iter() {
            let desc = device.device_descriptor().map_err(usb_err)?;
            if desc.vendor_id() == SONY_VID && desc.product_id() == PS3MCA_PID {
                found = Some(device);
                break;
            }
        }
        let device = found.ok_or_else(|| {
            "Could not find the PS3 Memory Card Adaptor.\nPlease make sure it is connected to a USB port."
                .to_string()
        })?;
        let desc = device.device_descriptor().map_err(usb_err)?;
        let handle = device.open().map_err(|e| {
            format!(
                "Could not open the PS3 Memory Card Adaptor ({e}). macOS may need a USB permission prompt."
            )
        })?;

        let _ = handle.set_active_configuration(1);
        if handle.kernel_driver_active(0).unwrap_or(false) {
            let _ = handle.detach_kernel_driver(0);
        }
        handle.claim_interface(0).map_err(|e| {
            format!("Could not claim USB interface 0 ({e}).")
        })?;

        let (in_ep, out_ep) = endpoints(&handle)?;
        let identity = identity_from(&device, &desc, String::new(), String::new());

        Ok(Self {
            handle,
            in_ep,
            out_ep,
            identity,
        })
    }

    fn detect_ps1_card(&mut self) -> Result<(), String> {
        let cmd = [0xAAu8, 0x40];
        self.handle
            .write_bulk(self.out_ep, &cmd, TIMEOUT)
            .map_err(|e| format!("USB write failed while detecting card ({e})."))?;
        let mut buf = [0u8; 8];
        let n = self
            .handle
            .read_bulk(self.in_ep, &mut buf, TIMEOUT)
            .map_err(|e| format!("USB read failed while detecting card ({e})."))?;
        if n != 2 || buf[0] != 0x55 || buf[1] != 0x01 {
            return Err("No PS1 memory card detected!".into());
        }
        Ok(())
    }

    fn read_frame(&mut self, frame: u16) -> Result<[u8; FRAME_SIZE], String> {
        let cmd = read_frame_command(frame);
        self.handle
            .write_bulk(self.out_ep, &cmd, TIMEOUT)
            .map_err(usb_err)?;
        let mut buf = [0u8; 256];
        let n = self
            .handle
            .read_bulk(self.in_ep, &mut buf, TIMEOUT)
            .map_err(usb_err)?;
        if n != READ_CMD_LEN || buf[0] != 0x55 || buf[1] != 0x5A {
            return Err(format!("Bad frame {frame} response ({n} bytes)."));
        }
        let mut frame_data = [0u8; FRAME_SIZE];
        frame_data.copy_from_slice(&buf[14..14 + FRAME_SIZE]);
        Ok(frame_data)
    }
}

impl Drop for Adaptor {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(0);
    }
}

fn read_frame_command(frame: u16) -> [u8; READ_CMD_LEN] {
    let mut cmd = [0u8; READ_CMD_LEN];
    cmd[0] = 0xAA;
    cmd[1] = 0x42;
    cmd[2] = (READ_CMD_LEN - 4) as u8;
    cmd[3] = 0x00;
    cmd[4] = 0x81;
    cmd[5] = 0x52;
    cmd[8] = (frame >> 8) as u8;
    cmd[9] = (frame & 0xFF) as u8;
    cmd
}

fn endpoints(handle: &DeviceHandle<Context>) -> Result<(u8, u8), String> {
    let config = handle
        .device()
        .config_descriptor(0)
        .map_err(usb_err)?;
    let mut in_ep = None;
    let mut out_ep = None;
    for interface in config.interfaces() {
        for desc in interface.descriptors() {
            for ep in desc.endpoint_descriptors() {
                if ep.transfer_type() != TransferType::Bulk {
                    continue;
                }
                match ep.direction() {
                    Direction::In => in_ep = Some(ep.address()),
                    Direction::Out => out_ep = Some(ep.address()),
                }
            }
        }
    }
    Ok((
        in_ep.unwrap_or(0x81),
        out_ep.unwrap_or(0x02),
    ))
}

fn identity_from(
    device: &rusb::Device<Context>,
    desc: &rusb::DeviceDescriptor,
    manufacturer: String,
    product: String,
) -> AdaptorIdentity {
    let v = desc.device_version();
    AdaptorIdentity {
        vid: desc.vendor_id(),
        pid: desc.product_id(),
        bcd_device: format!("{}.{}.{}", v.major(), v.minor(), v.sub_minor()),
        manufacturer,
        product,
        bus: device.bus_number(),
        address: device.address(),
    }
}

fn usb_err<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_command_matches_memcardrex() {
        let cmd = read_frame_command(0x0042);
        assert_eq!(&cmd[0..6], &[0xAA, 0x42, 140, 0x00, 0x81, 0x52]);
        assert_eq!(cmd[8], 0x00);
        assert_eq!(cmd[9], 0x42);
        assert_eq!(cmd.len(), 144);
        let last = read_frame_command(1023);
        assert_eq!(last[8], 0x03);
        assert_eq!(last[9], 0xFF);
    }

    #[test]
    #[ignore = "requires the PS3 Memory Card Adaptor"]
    fn live_sony_adaptor_present() {
        let ctx = rusb::Context::new().expect("libusb context");
        let found = find_identity(&ctx).expect("enumerate USB");
        let id = found.expect("PS3 Memory Card Adaptor 054C:02EA should be plugged in");
        assert_eq!(id.vid, SONY_VID);
        assert_eq!(id.pid, PS3MCA_PID);
        eprintln!(
            "adaptor vid={:04X} pid={:04X} bcd={} bus={} addr={} manufacturer={:?} product={:?}",
            id.vid, id.pid, id.bcd_device, id.bus, id.address, id.manufacturer, id.product
        );
    }

    #[test]
    #[ignore = "requires a PS1 card in the adaptor"]
    fn live_read_ps1_card() {
        let ctx = rusb::Context::new().expect("libusb context");
        let (bytes, identity) = read_card(&ctx, |_| {}).expect("read adaptor");
        assert_eq!(bytes.len(), crate::card::CARD_SIZE);
        let card = crate::card::Ps1Card::open_from(
            &bytes,
            "adaptor",
            false,
            crate::card::CardSource::Usb,
        )
        .expect("parse dumped card");
        let view = card.view();
        eprintln!(
            "live card {:04X}:{:04X} used={} saves={} titles={:?}",
            identity.vid,
            identity.pid,
            view.used_blocks,
            view.saves.len(),
            view.saves
                .iter()
                .map(|s| format!(
                    "{} [{}] {} {:?}",
                    s.title, s.region, s.prod_code, s.linked_slots
                ))
                .collect::<Vec<_>>()
        );
        assert_eq!(view.slots.len(), 15);
    }
}
