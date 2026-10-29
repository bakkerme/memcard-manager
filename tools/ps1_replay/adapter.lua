-- GPL-3.0-or-later. Research-only adapter.
-- Guest RAM pokes run only when PS1_REPLAY_GUEST_WRITES=1.
-- Writes use PCSX.getMemoryAsFile():writeU8At/writeU16At/writeU32At (value, then address).
-- This file never writes the memory-card image; the game saves itself.
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
local sticky_pokes = {}
local function guest_writes_enabled()
    return os.getenv('PS1_REPLAY_GUEST_WRITES') == '1'
end
local function required_number(raw, what)
    local n = tonumber(raw)
    if n == nil then error('Missing ' .. what) end
    return n
end
local function validate_span(address, size, value)
    if type(address) ~= 'number' or type(size) ~= 'number' or address ~= math.floor(address) or size ~= math.floor(size) then
        error('Poke address and size must be integers')
    end
    if size ~= 1 and size ~= 2 and size ~= 4 then error('Poke size must be 1, 2, or 4') end
    if address < 0x80000000 or address + size > 0x80200000 then
        error('Poke must stay inside guest RAM 0x80000000-0x80200000')
    end
    if address % size ~= 0 then error('Poke address must be aligned to its size') end
    if value ~= nil then
        local limit = ({[1] = 0xff, [2] = 0xffff, [4] = 0xffffffff})[size]
        if type(value) ~= 'number' or value ~= math.floor(value) or value < 0 or value > limit then
            error('Poke value does not fit in the requested size')
        end
    end
end
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
local function read_width(m, address, size)
    if size == 1 then return tonumber(m:readU8At(address)) end
    if size == 2 then return tonumber(m:readU16At(address)) end
    return tonumber(m:readU32At(address))
end
local function write_width(m, address, size, value)
    -- File helpers take the value first, then the address. Avoid getMemPtr.
    if size == 1 then m:writeU8At(value, address)
    elseif size == 2 then m:writeU16At(value, address)
    else m:writeU32At(value, address) end
end
local function json_num(n)
    return string.format('%.0f', tonumber(n))
end
local function append_memory_write(entry)
    -- Vsync reapplies are not separate host commands, so the adapter records them.
    local parts = {
        '"address":' .. json_num(entry.address),
        '"after":' .. json_num(entry.after),
        '"before":' .. json_num(entry.before),
        '"frame":' .. json_num(entry.frame),
        '"reapplied":' .. (entry.reapplied and 'true' or 'false'),
        '"size":' .. json_num(entry.size),
        '"skipped":' .. (entry.skipped and 'true' or 'false'),
        '"sticky":' .. (entry.sticky and 'true' or 'false'),
        '"timestamp":' .. json_num(entry.timestamp),
        '"value":' .. json_num(entry.value),
    }
    if entry.when then
        parts[#parts + 1] = '"when":{"address":' .. json_num(entry.when.address)
            .. ',"op":' .. quote(entry.when.op)
            .. ',"size":' .. json_num(entry.when.size)
            .. ',"value":' .. json_num(entry.when.value) .. '}'
    end
    local f = assert(io.open(artifact('memory-writes.jsonl'), 'a'))
    assert(f:write('{' .. table.concat(parts, ',') .. '}\n'))
    assert(f:close())
end
local function condition_holds(m, when)
    if when == nil then return true end
    local got = read_width(m, when.address, when.size)
    if when.op == 'eq' then return got == when.value end
    if when.op == 'ne' then return got ~= when.value end
    error('Poke condition must be eq or ne')
end
local function apply_poke(spec, reapplied)
    local m = PCSX.getMemoryAsFile()
    local ok, result = pcall(function()
        local before = read_width(m, spec.address, spec.size)
        local skipped = not condition_holds(m, spec.when)
        if not skipped then write_width(m, spec.address, spec.size, spec.value) end
        local after = read_width(m, spec.address, spec.size)
        if not skipped and after ~= spec.value then
            error(string.format('Poke did not land at %.0f: before=%s after=%s', spec.address, tostring(before), tostring(after)))
        end
        local outcome = {
            address = spec.address, size = spec.size, value = spec.value,
            before = before, after = after, frame = state.frame,
            sticky = spec.sticky and true or false, skipped = skipped,
            reapplied = reapplied and true or false, timestamp = os.time(),
        }
        if spec.when then outcome.when = spec.when end
        return outcome
    end)
    pcall(function() m:close() end)
    if not ok then error(tostring(result), 0) end
    return result
end
local function poke_spec(p)
    if not guest_writes_enabled() then error('Guest memory writes are disabled') end
    local address = required_number(p.address, 'poke address')
    local size = required_number(p.size, 'poke size')
    local value = required_number(p.value, 'poke value')
    validate_span(address, size, value)
    local function present(v)
        return v ~= nil and v ~= ''
    end
    local when = nil
    if present(p.when_op) or present(p.when_address) or present(p.when_size) or present(p.when_value) then
        if not (present(p.when_op) and present(p.when_address) and present(p.when_size) and present(p.when_value)) then
            error('Poke condition needs when_op, when_address, when_size, and when_value')
        end
        if p.when_op ~= 'eq' and p.when_op ~= 'ne' then error('Poke condition must be eq or ne') end
        local waddress = required_number(p.when_address, 'condition address')
        local wsize = required_number(p.when_size, 'condition size')
        local wvalue = required_number(p.when_value, 'condition value')
        validate_span(waddress, wsize, wvalue)
        when = {op = p.when_op, address = waddress, size = wsize, value = wvalue}
    end
    return {address = address, size = size, value = value, sticky = p.sticky == '1', when = when}
end
local function arm_sticky(spec)
    local kept = {}
    for i = 1, #sticky_pokes do
        local old = sticky_pokes[i]
        if not (old.address == spec.address and old.size == spec.size) then kept[#kept + 1] = old end
    end
    kept[#kept + 1] = spec
    sticky_pokes = kept
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
    elseif p.action == 'poke' then
        PCSX.pauseEmulator()
        local spec = poke_spec(p)
        local entry = apply_poke(spec, false)
        if spec.sticky then arm_sticky(spec) end
        -- The coordinator records this one-shot. Vsync reapplies append memory-writes.jsonl.
        finish(entry)
    elseif p.action == 'poke-clear' then
        if not guest_writes_enabled() then error('Guest memory writes are disabled') end
        local address = nil
        if p.address then address = required_number(p.address, 'poke address') end
        local kept = {}
        if address ~= nil then
            for i = 1, #sticky_pokes do
                if sticky_pokes[i].address ~= address then kept[#kept + 1] = sticky_pokes[i] end
            end
        end
        sticky_pokes = kept
        finish({cleared = true, remaining = #sticky_pokes})
    elseif p.action == 'watch' then
        -- Caller-supplied address. Do not use the Legends 2 fields in values().
        PCSX.pauseEmulator()
        local address = required_number(p.address, 'watch address')
        local size = required_number(p.size, 'watch size')
        validate_span(address, size, nil)
        local watched
        local m = PCSX.getMemoryAsFile()
        local ok, err = pcall(function() watched = read_width(m, address, size) end)
        pcall(function() m:close() end)
        if not ok then error(tostring(err), 0) end
        local result = {address = address, size = size, value = watched}
        if p.name and p.name ~= '' then
            assert(p.name:match('^[%w_.-]+$'), 'Invalid watch name')
            result.name = p.name
        end
        finish(result)
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
    if #sticky_pokes > 0 and guest_writes_enabled() then
        local kept = {}
        for i = 1, #sticky_pokes do
            local ok, entry = pcall(apply_poke, sticky_pokes[i], true)
            if ok then
                local logged, log_err = pcall(append_memory_write, entry)
                if logged then kept[#kept + 1] = sticky_pokes[i]
                else print('REPLAY_POKE_DROPPED ' .. tostring(log_err)) end
            else
                print('REPLAY_POKE_DROPPED ' .. tostring(entry))
            end
        end
        sticky_pokes = kept
    end
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
