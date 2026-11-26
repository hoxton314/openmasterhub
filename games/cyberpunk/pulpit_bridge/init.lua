-- PULPIT bridge core: hub <-> Cyberpunk 2077. Stays tiny; logic.lua holds stats + commands
-- and is re-loaded by the "reload" command, so changes need no CET "Reload all mods".
-- out: stats.json (every 0.25 s), result.json (last 10 command results)
-- in:  cmd.txt lines "<seq>\t<cmd>\t<arg>\n" written by PULPIT; seq only grows.
PB = PB or {}
PB.last, PB.t, PB.tc, PB.ready, PB.results = -1, 0, 0, false, {}

local function q(s) return '"' .. tostring(s):gsub('[%c"\\]', function(c) return string.format("\\u%04x", c:byte()) end) .. '"' end
local function enc(v)
  local t = type(v)
  if t == "table" then
    if next(v) == nil then return "[]" end
    if #v > 0 then local o = {} for i, x in ipairs(v) do o[i] = enc(x) end return "[" .. table.concat(o, ",") .. "]" end
    local o = {} for k, x in pairs(v) do o[#o + 1] = q(k) .. ":" .. enc(x) end return "{" .. table.concat(o, ",") .. "}"
  elseif t == "number" then
    if v ~= v or v == math.huge or v == -math.huge then return "null" end
    if v == math.floor(v) and math.abs(v) < 1e15 then return string.format("%.0f", v) end
    return string.format("%.6g", v)
  elseif t == "boolean" then return tostring(v)
  elseif v == nil then return "null" end
  return q(v)
end
PB.enc = enc
function PB.try(f, ...) local ok, r = pcall(f, ...) if ok then return r end return nil end
function PB.write(name, text) local f = io.open(name, "w") if not f then return end f:write(text) f:close() end

PB.C = PB.C or {}
local function loadLogic()
  local f, err = loadfile("logic.lua")
  if not f then return false, err end
  local ok, e = pcall(f)
  return ok, e
end
local function core(cmd, arg)
  if cmd == "ping" then return "pong" end
  if cmd == "reload" then local ok, e = loadLogic() if not ok then error(e) end return "reloaded" end
  if cmd == "eval" then local f, err = loadfile(arg) if not f then error(err) end return f() end
end

local function run(seq, cmd, arg)
  local ok, r
  if cmd == "ping" or cmd == "reload" or cmd == "eval" then ok, r = pcall(core, cmd, arg)
  elseif PB.C[cmd] then ok, r = pcall(PB.C[cmd], arg)
  else ok, r = false, "unknown command " .. cmd end
  table.insert(PB.results, 1, { seq = seq, cmd = cmd, ok = ok, value = ok and r or tostring(r) })
  while #PB.results > 10 do table.remove(PB.results) end
  PB.write("result.json", enc(PB.results))
end

local function poll(init)
  local f = io.open("cmd.txt", "r") if not f then return end
  local data = f:read("*a") f:close()
  for line in data:gmatch("([^\n]*)\n") do
    local seq, cmd, arg = line:match("^(%d+)\t([%w_]+)\t?(.*)$")
    seq = tonumber(seq)
    if seq and seq > PB.last then
      PB.last = seq
      if not init then run(seq, cmd, arg) end
    end
  end
end

registerForEvent("onInit", function()
  poll(true)
  local ok, e = loadLogic()
  PB.logic_err = (not ok) and tostring(e) or nil
  PB.ready = true
  PB.write("hello.json", enc({ started = os.time(), last = PB.last, logic = ok, err = PB.logic_err }))
end)

registerForEvent("onUpdate", function(dt)
  if not PB.ready then return end
  PB.tc = PB.tc + dt
  if PB.tc >= 0.05 then PB.tc = 0 poll(false) end
  if PB.tick then PB.try(PB.tick, dt) end
  PB.t = PB.t + dt
  if PB.t >= 0.25 then
    PB.t = 0
    local s = (PB.stats and PB.try(PB.stats)) or { ingame = false, err = true }
    s.at, s.last, s.logic_err = os.clock(), PB.last, PB.logic_err
    PB.write("stats.json", enc(s))
  end
end)

registerForEvent("onShutdown", function() PB.write("stats.json", enc({ ingame = false, closed = true })) end)
return PB
