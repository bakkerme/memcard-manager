-- GPL-3.0-or-later. Research-only adapter; no guest memory/card write commands.
local function quote(s)
    return '"' .. s:gsub('[%z\1-\31\\"]', function(c)
        if c == '\\' then return '\\\\' end
        if c == '"' then return '\\"' end
        return string.format('\\u%04x', string.byte(c))
    end) .. '"'
end
local function json(v)
    if type(v) == 'string' then return quote(v) end
    if type(v) == 'number' or type(v) == 'boolean' then return tostring(v) end
    if v == nil then return 'null' end
    local parts = {}
    for k, x in pairs(v) do parts[#parts + 1] = quote(tostring(k)) .. ':' .. json(x) end
    return '{' .. table.concat(parts, ',') .. '}'
end
local function write(path, data)
    local f = assert(io.open(path, 'wb')); assert(f:write(data)); assert(f:close())
end
local pad = PCSX.SIO0.slots[1].pads[1]
local buttons = PCSX.CONSTS.PAD.BUTTON
local function release()
    for _, button in pairs(buttons) do pad.clearOverride(button) end
end
local state = {ready = true, frame = 0, busy = false}
local target, pending
local function finish(result)
    state.busy = false
    state.result = result or {}
    state.result.frame = state.frame
    state.done = state.id
    target = nil
end
local function fail(err)
    release(); PCSX.pauseEmulator(); finish({error = tostring(err)})
end
local function artifact(name)
    assert(name and name:match('^[%w_.-]+$'), 'Invalid artifact name')
    return '/work/' .. os.getenv('REPLAY_STAGE') .. '/' .. name
end
local function snapshot(name)
    local shot = PCSX.GPU.takeScreenShot()
    assert(shot.width > 0 and shot.height > 0, 'Empty GPU screenshot')
    write(artifact(name .. '.raw'), tostring(shot.data))
    local meta = {width = tonumber(shot.width), height = tonumber(shot.height),
        bpp = tonumber(shot.bpp) == 0 and 16 or 24, frame = state.frame}
    write(artifact(name .. '.json'), json(meta))
    return meta
end
local function values()
    local m = PCSX.getMemoryAsFile()
    local r = {map = m:readU8At(0x8009c808), secondary = m:readU8At(0x8009c809),
        difficulty = m:readU8At(0x8009c80e), zenny = m:readU32At(0x8009c820),
        helmet = m:readU8At(0x8008c252), shoes = m:readU8At(0x8008c253),
        armor = m:readU8At(0x8008c254), part1 = m:readU8At(0x8008c258),
        part2 = m:readU8At(0x8008c259), part3 = m:readU8At(0x8008c25a),
        pc = tonumber(PCSX.getRegisters().pc)}
    m:close(); return r
end
local function execute(p)
    if p.action == 'frames' or p.action == 'press' then
        local count = assert(tonumber(p.frames), 'Missing frame count')
        assert(count >= 1 and count <= 36000 and count == math.floor(count), 'Invalid frame count')
        release()
        if p.action == 'press' then pad.setOverride(assert(buttons[p.button], 'Unknown button')) end
        pending = {requestedFrames = count, startFrame = state.frame}
        target = state.frame + count
        PCSX.resumeEmulator()
    elseif p.action == 'resume' then
        release(); target = nil; finish({unboundedDiagnosticRun = true}); PCSX.resumeEmulator()
    elseif p.action == 'release' then
        release(); PCSX.pauseEmulator(); finish({released = true})
    elseif p.action == 'capture' then
        PCSX.pauseEmulator(); finish(snapshot(p.name))
    elseif p.action == 'memory' then
        PCSX.pauseEmulator()
        local m = PCSX.getMemoryAsFile()
        local start, length = assert(tonumber(p.address)), assert(tonumber(p.length))
        assert(start >= 0x80000000 and start + length <= 0x80200000 and length > 0, 'RAM range required')
        write(artifact(p.name .. '.bin'), tostring(m:readAt(length, start)))
        m:close(); finish(values())
    elseif p.action == 'values' then finish(values())
    elseif p.action == 'diagnostics' then
        PCSX.pauseEmulator()
        local r = PCSX.getRegisters()
        local registers = {pc = tonumber(r.pc), gpr = {}, cp0 = {}}
        for i=0,31 do
            registers.gpr[i] = tonumber(r.GPR.r[i]); registers.cp0[i] = tonumber(r.CP0.r[i])
        end
        write(artifact(p.name .. '.json'), json(registers))
        write(artifact('state-schema.proto'), PCSX.getSaveStateProtoSchema())
        finish(values())
    elseif p.action == 'settings' then
        local e = PCSX.settings.emulator
        finish({Dynarec = e.Dynarec, HardwareRenderer = e.HardwareRenderer,
                AutoUpdate = e.AutoUpdate, LinearFiltering = e.LinearFiltering,
                FastBoot = e.FastBoot, SpuIrq = e.SpuIrq})
    elseif p.action == 'save' then
        release(); PCSX.pauseEmulator()
        write(artifact(p.name), tostring(PCSX.createSaveState())); finish(values())
    elseif p.action == 'restore' then
        release(); PCSX.pauseEmulator()
        local f = Support.File.open('/work/baseline/state.bin')
        PCSX.loadSaveState(f); f:close(); state.frame = 0; finish(values())
    elseif p.action == 'quit' then release(); finish({}); PCSX.quit(0)
    elseif p.action == 'restore_diagnostic' then
        release(); PCSX.pauseEmulator()
        local f = Support.File.open('/inputs/diagnostic.state')
        PCSX.loadSaveState(f); f:close(); state.frame = 0; finish(values())
    elseif p.action == 'restore_saved' then
        release(); PCSX.pauseEmulator()
        local f = Support.File.open(artifact(p.name))
        PCSX.loadSaveState(f); f:close(); state.frame = 0; finish(values())
    elseif p.action == 'spu_irq' then
        PCSX.settings.emulator.SpuIrq = true; finish({SpuIrq = PCSX.settings.emulator.SpuIrq})
    elseif p.action == 'reset_normal' or p.action == 'reset_fast' then
        release(); PCSX.settings.emulator.FastBoot = p.action == 'reset_fast'
        PCSX.hardResetEmulator(); state.frame = 0; finish({reset = true})
    else error('Unknown action') end
end
-- Keep listeners alive. Pause immediately at guest vsync; defer captures and
-- other complex operations to the emulator's safe main-loop context.
REPLAY_LISTENER = PCSX.Events.createEventListener('GPU::Vsync', function()
    state.frame = state.frame + 1
    if target and state.frame >= target then
        PCSX.pauseEmulator(); release(); target = nil
        PCSX.nextTick(function()
            pending.actualFrames = state.frame - pending.startFrame
            finish(pending)
        end)
    end
end)
PCSX.WebServer = PCSX.WebServer or {}
PCSX.WebServer.Handlers = PCSX.WebServer.Handlers or {}
PCSX.WebServer.Handlers.replay_status = function() return json(state) end
PCSX.WebServer.Handlers.replay_command = function(request)
    local p = {}
    for k, v in request.urlData.query:gmatch('([^&=]+)=([^&]*)') do
        p[k] = v:gsub('%%(%x%x)', function(h) return string.char(tonumber(h, 16)) end)
    end
    -- Emergency release interrupts an outstanding frame wait.
    if state.busy and p.action ~= 'release' then return json({error = 'Adapter busy'}) end
    state.id = p.id; state.busy = true; state.result = nil
    PCSX.nextTick(function()
        local ok, err = pcall(execute, p)
        if not ok then fail(err) end
    end)
    return json({accepted = p.id})
end
release()
PCSX.pauseEmulator()
print('REPLAY_ADAPTER_READY')
