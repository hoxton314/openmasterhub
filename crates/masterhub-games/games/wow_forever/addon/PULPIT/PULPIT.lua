-- PULPIT: the desk hub's helper inside WoW Forever.
--  * Binds free keys to the bar buttons the hub's keys press (only keys nothing uses).
--  * Saves what sits on those bars (icon file IDs, names, base cooldowns) and the
--    character's state (level, XP, rested, money, zone, quests) into PulpitDB, which the
--    game writes to disk on /reload, logout and exit. The hub reads that file.
-- It never acts on its own in combat and sends nothing anywhere.

local ADDON = ...
PulpitDB = PulpitDB or {}

-- Hub bar keys: bar button action -> the key the hub presses (games/wow_forever mod.rs DEFAULTS).
-- Each action takes the first of its keys that nothing uses yet.
local WANT = {
  { "MULTIACTIONBAR1BUTTON10", "CTRL-0", "ALT-SHIFT-1", "ALT-CTRL-1", "CTRL-SHIFT-1" },
  { "MULTIACTIONBAR1BUTTON11", "CTRL--", "ALT-SHIFT-2", "ALT-CTRL-2", "CTRL-SHIFT-2" },
  { "MULTIACTIONBAR1BUTTON12", "CTRL-=", "ALT-SHIFT-3", "ALT-CTRL-3", "CTRL-SHIFT-3" },
  { "MULTIACTIONBAR2BUTTON1", "ALT-1", "ALT-SHIFT-4" },
  { "MULTIACTIONBAR2BUTTON2", "ALT-2", "ALT-SHIFT-5" },
  { "MULTIACTIONBAR2BUTTON3", "ALT-3", "ALT-SHIFT-6" },
  { "MULTIACTIONBAR2BUTTON4", "ALT-4", "ALT-SHIFT-7" },
  { "MULTIACTIONBAR2BUTTON5", "ALT-5", "ALT-SHIFT-8" },
  { "MULTIACTIONBAR2BUTTON6", "ALT-6", "ALT-SHIFT-9" },
}

-- A tiny JSON writer (strings, numbers, booleans, arrays, objects).
local function json(v)
  local t = type(v)
  if t == "nil" then return "null"
  elseif t == "boolean" then return v and "true" or "false"
  elseif t == "number" then
    if v ~= v or v == math.huge or v == -math.huge then return "null" end
    return string.format("%.14g", v)
  elseif t == "string" then
    return '"' .. v:gsub('[%c"\\]', function(c)
      local map = { ['"'] = '\\"', ['\\'] = '\\\\', ['\n'] = '\\n', ['\r'] = '\\r', ['\t'] = '\\t' }
      return map[c] or string.format("\\u%04x", c:byte())
    end) .. '"'
  elseif t == "table" then
    local n = #v
    if n > 0 or next(v) == nil then
      local out = {}
      for i = 1, n do out[i] = json(v[i]) end
      return "[" .. table.concat(out, ",") .. "]"
    end
    local out = {}
    for k, val in pairs(v) do out[#out + 1] = json(tostring(k)) .. ":" .. json(val) end
    return "{" .. table.concat(out, ",") .. "}"
  end
  return "null"
end

local function bindFree()
  if InCombatLockdown() then return end
  local made, kept, taken = {}, {}, {}
  for _, w in ipairs(WANT) do
    local action = w[1]
    local have = GetBindingKey(action)
    if have then
      kept[action] = have
    else
      for i = 2, #w do
        local key = w[i]
        local holder = GetBindingAction(key) or ""
        if holder == "" and SetBinding(key, action) then
          made[action] = key
          break
        end
        taken[key] = holder ~= "" and holder or "SetBinding refused"
      end
    end
  end
  if next(made) then
    SaveBindings(GetCurrentBindingSet())
  end
  PulpitDB.made = made
  PulpitDB.kept = kept
  PulpitDB.taken = taken
end

-- The buttons of a bar by its binding prefix.
local BAR_BUTTONS = {
  MULTIACTIONBAR1BUTTON = "MultiBarBottomLeftButton",
  MULTIACTIONBAR2BUTTON = "MultiBarBottomRightButton",
  ACTIONBUTTON = "ActionButton",
}

local function slotInfo(slot)
  if not slot or not HasAction(slot) then return { empty = true } end
  local kind, id = GetActionInfo(slot)
  local e = { kind = kind, id = id, icon = GetActionTexture(slot) }
  if kind == "spell" and C_Spell and C_Spell.GetSpellName then
    e.name = C_Spell.GetSpellName(id)
    if GetSpellBaseCooldown then e.cd = (GetSpellBaseCooldown(id) or 0) / 1000 end
  elseif kind == "item" and C_Item and C_Item.GetItemNameByID then
    e.name = C_Item.GetItemNameByID(id)
  elseif kind == "macro" then
    e.name = GetActionText(slot) or (GetMacroInfo and GetMacroInfo(id))
  elseif kind == "summonmount" and C_MountJournal then
    e.name = C_MountJournal.GetMountInfoByID(id)
  end
  e.name = e.name or GetActionText(slot)
  return e
end

local function snapshot()
  local bars = {}
  for prefix, frame in pairs(BAR_BUTTONS) do
    local list = {}
    for i = 1, 12 do
      local b = _G[frame .. i]
      list[i] = slotInfo(b and b.action)
      list[i].bind = GetBindingKey(prefix .. i)
    end
    bars[prefix] = list
  end
  local quests = {}
  if C_QuestLog and C_QuestLog.GetNumQuestLogEntries then
    for i = 1, C_QuestLog.GetNumQuestLogEntries() do
      local q = C_QuestLog.GetInfo(i)
      if q and not q.isHeader and q.questID then
        local objs = {}
        for _, o in ipairs(C_QuestLog.GetQuestObjectives(q.questID) or {}) do
          objs[#objs + 1] = { text = o.text, done = o.finished }
        end
        quests[#quests + 1] = { id = q.questID, title = q.title, level = q.level,
          done = C_QuestLog.IsComplete(q.questID), watched = C_QuestLog.GetQuestWatchType(q.questID) ~= nil,
          objectives = objs }
      end
    end
  end
  local map = C_Map and C_Map.GetBestMapForUnit and C_Map.GetBestMapForUnit("player")
  local pos = map and C_Map.GetPlayerMapPosition and C_Map.GetPlayerMapPosition(map, "player")
  local _, class = UnitClass("player")
  local snap = {
    time = time(), name = UnitName("player"), realm = GetRealmName(), class = class, faction = UnitFactionGroup("player"),
    level = UnitLevel("player"), xp = UnitXP("player"), xpmax = UnitXPMax("player"),
    rested = GetXPExhaustion() or 0, money = GetMoney(),
    zone = GetZoneText(), subzone = GetSubZoneText(), map = map,
    x = pos and pos.x, y = pos and pos.y,
    bars = bars, quests = quests, made = PulpitDB.made, kept = PulpitDB.kept, taken = PulpitDB.taken,
  }
  -- One JSON string: easy to read outside the game.
  PulpitDB.json = json(snap)
end

local applyLayout -- defined below

local f = CreateFrame("Frame")
f:RegisterEvent("PLAYER_ENTERING_WORLD")
f:RegisterEvent("PLAYER_LOGOUT")
f:RegisterEvent("PLAYER_REGEN_ENABLED")
f:SetScript("OnEvent", function(_, event)
  if event == "PLAYER_ENTERING_WORLD" then
    -- The hub reads XP, loot and money from the chat log and fights from the combat log; the
    -- game turns chat logging off again on a reload, so switch both on every time.
    if LoggingChat and not LoggingChat() then LoggingChat(true) end
    if LoggingCombat and not LoggingCombat() then LoggingCombat(true) end
    bindFree()
    snapshot()
    if C_Timer then C_Timer.After(3, applyLayout) end
  elseif event == "PLAYER_REGEN_ENABLED" then
    -- Out of combat: bindings pending from a login in combat, and a fresh snapshot.
    if not PulpitDB.made then bindFree() end
    snapshot()
  else
    snapshot()
  end
end)

-- Our screen arrangement for the parts Edit Mode does not place (add-on windows) and the chat
-- size, applied a moment after login so other add-ons have built their frames. Units are
-- UIParent's (2133 x 1200 at this screen). `/pulpit layout off` stops it; `on` brings it back.
local LAYOUT = {
  -- Left column: the chat at the bottom, the levelling guide right above it; the top-left
  -- corner stays free for party frames in dungeons.
  chat = { w = 440, h = 200, x = 35, y = 145 },
  guide = { frame = "RXPFrame", point = "BOTTOMLEFT", x = 35, y = 395 },
}

applyLayout = function()
  if PulpitDB.layoutOff or InCombatLockdown() then return end
  local c = LAYOUT.chat
  if ChatFrame1 and FCF_SavePositionAndDimensions then
    ChatFrame1:ClearAllPoints()
    ChatFrame1:SetPoint("BOTTOMLEFT", UIParent, "BOTTOMLEFT", c.x, c.y)
    ChatFrame1:SetSize(c.w, c.h)
    pcall(FCF_SavePositionAndDimensions, ChatFrame1)
  end
  -- TomTom's coordinate box sits mid-screen by default; the minimap already shows the
  -- coordinates. Switched off in TomTom's own profile once (the user can turn it back on there).
  if TomTom and TomTom.profile and TomTom.profile.block and not PulpitDB.tomtomBlockDone then
    TomTom.profile.block.enable = false
    if TomTom.ShowHideCoordBlock then pcall(TomTom.ShowHideCoordBlock, TomTom) end
    PulpitDB.tomtomBlockDone = true
  end
  local g = LAYOUT.guide
  local f = _G[g.frame]
  if f then
    f:ClearAllPoints()
    f:SetPoint(g.point, UIParent, g.point, g.x, g.y)
  end
end

-- The screen layout: every Edit Mode system's anchor and settings, plus the UI scale and the
-- screen size, for designing a layout outside the game.
local function dumpLayout()
  local out = { screen = { GetPhysicalScreenSize() }, uiscale = UIParent:GetEffectiveScale(),
    width = UIParent:GetWidth(), height = UIParent:GetHeight() }
  local ok, layouts = pcall(C_EditMode.GetLayouts)
  if ok and layouts then out.layouts = layouts end
  if EditModeManagerFrame and EditModeManagerFrame.GetActiveLayoutInfo then
    local ok2, active = pcall(EditModeManagerFrame.GetActiveLayoutInfo, EditModeManagerFrame)
    if ok2 then out.active = active end
  end
  -- Where the frames really are (bottom-left origin, UIParent units).
  local frames = {}
  for _, name in ipairs({ "PlayerFrame", "TargetFrame", "FocusFrame", "PartyFrame", "PetFrame", "MainMenuBar", "MainActionBar",
      "MultiBarBottomLeft", "MultiBarBottomRight", "MultiBarRight", "MultiBarLeft", "StanceBar", "PetActionBar",
      "ChatFrame1", "MinimapCluster", "BuffFrame", "DebuffFrame", "ObjectiveTrackerFrame", "QuestWatchFrame",
      "MicroMenuContainer", "BagsBar", "MainStatusTrackingBarContainer", "PlayerCastingBarFrame", "DurabilityFrame",
      "Questie_BaseFrame", "RXPFrame", "LootFrame", "GameTooltipDefaultContainer", "DamageMeter", "ExtraAbilityContainer" }) do
    local f = _G[name]
    if f and f.GetRect then
      local l, b, w, h = f:GetRect()
      frames[name] = { l = l, b = b, w = w, h = h, shown = f:IsShown(), scale = f:GetEffectiveScale() }
    end
  end
  out.frames = frames
  PulpitDB.layout = json(out)
  print("|cffffd100PULPIT|r: layout saved on next /reload.")
end

SLASH_PULPIT1 = "/pulpit"
SlashCmdList.PULPIT = function(msg)
  if msg == "layout" then return dumpLayout() end
  if msg == "layout off" then PulpitDB.layoutOff = true; print("|cffffd100PULPIT|r: layout left alone from now on."); return end
  if msg == "layout on" then PulpitDB.layoutOff = nil; applyLayout(); print("|cffffd100PULPIT|r: layout applied."); return end
  bindFree()
  snapshot()
  local n = 0
  for _ in pairs(PulpitDB.made or {}) do n = n + 1 end
  print("|cffffd100PULPIT|r: snapshot saved on next /reload. Keys bound this time: " .. n)
end

-- PULPIT strip: a row of small coloured cells in the top-left corner that the desk hub reads
-- from the screen a few times a second (the game writes its logs only every few minutes, so
-- files cannot carry live data). Display only: nothing comes back into the game.
--
-- Layout: [magenta][cyan][seq][60 data symbols][checksum], each cell CELL x CELL physical
-- pixels. A symbol is 6 bits, 2 per colour channel at levels 0/85/170/255, so small colour
-- shifts do not matter. Data (42 bytes, big-endian):
--   (45 bytes) 0 level | 1-3 xp | 4-6 xp max | 7-9 rested | 10-13 money (copper) | 14-15 uiMapID
--   16-17 x*65535 | 18-19 y*65535 | 20 flags (1 combat, 2 dead, 4 resting, 8 mounted)
--   21 health % | 22 power % | 23 target health % (255 none)
--   24-35 bar 1 buttons 1-12 cooldown left (s, 255 = none) | 36-41 bar 2 buttons 1-6
--   42 class id (ChrClasses) | 43 faction (0 Horde, 1 Alliance, 2 other) | 44 unused
-- The hub's decoder: crates/pulpit/src/games/wow_forever/strip.rs. (Kept in this file: a new TOC entry needs a game restart, a changed file only a /reload.)

local CELL = 6
local DATA = 45
local SYMBOLS = 60

local strip = CreateFrame("Frame", "PulpitStrip", UIParent)
strip:SetIgnoreParentScale(true)
strip:SetFrameStrata("TOOLTIP")
strip:SetFrameLevel(10000)
local cells = {}
local function layout()
  -- One unit = one physical pixel: the screen is 768 units high at scale 1.
  local _, h = GetPhysicalScreenSize()
  strip:SetScale(768 / h)
  strip:ClearAllPoints()
  strip:SetPoint("TOPLEFT", UIParent, "TOPLEFT", 0, 0)
  strip:SetSize((SYMBOLS + 4) * CELL, CELL)
  for i = 1, SYMBOLS + 4 do
    local t = cells[i]
    if not t then
      t = strip:CreateTexture(nil, "OVERLAY")
      t:SetSnapToPixelGrid(true)
      t:SetTexelSnappingBias(0)
      cells[i] = t
    end
    t:SetSize(CELL, CELL)
    t:ClearAllPoints()
    t:SetPoint("TOPLEFT", strip, "TOPLEFT", (i - 1) * CELL, 0)
  end
end
layout()

local function sym(i, s)
  local r, g, b = math.floor(s / 16) % 4, math.floor(s / 4) % 4, s % 4
  cells[i]:SetColorTexture(r / 3, g / 3, b / 3, 1)
end

local bytes = {}
local function put(at, value, n)
  value = math.max(0, math.floor(value or 0))
  for k = n - 1, 0, -1 do
    bytes[at + k] = value % 256
    value = math.floor(value / 256)
  end
end

local function cooldown(slot)
  if not slot or not HasAction(slot) then return 255 end
  local start, duration = GetActionCooldown(slot)
  if not start or not duration or duration <= 1.6 or start == 0 then return 0 end
  local left = start + duration - GetTime()
  if left <= 0 then return 0 end
  return math.min(254, math.ceil(left))
end

local function pct(v, m)
  if not v or not m or m <= 0 then return 0 end
  return math.min(100, math.floor(v / m * 100 + 0.5))
end

local seq = 0
-- Forever (like retail) hands add-ons "secret" numbers for some unit data (health in combat,
-- some cooldowns): arithmetic on them fails. Every field is computed on its own and falls back
-- to its "unknown" value, so one blocked value never blanks the strip.
local blocked = {}
local function field(name, at, n, fallback, fn)
  local ok, v = pcall(fn)
  if ok and type(v) == "number" then
    local ok2 = pcall(put, at, v, n)
    if ok2 then return end
  end
  blocked[name] = true
  put(at, fallback, n)
end

local function encode()
  for i = 0, DATA - 1 do bytes[i] = 0 end
  field("level", 0, 1, 0, function() return UnitLevel("player") end)
  field("xp", 1, 3, 0, function() return UnitXP("player") end)
  field("xpmax", 4, 3, 0, function() return UnitXPMax("player") end)
  field("rested", 7, 3, 0, function() return GetXPExhaustion() or 0 end)
  field("money", 10, 4, 0, function() return GetMoney() end)
  local map
  field("map", 14, 2, 0, function() map = C_Map.GetBestMapForUnit("player"); return map or 0 end)
  local pos
  pcall(function() pos = map and C_Map.GetPlayerMapPosition(map, "player") end)
  field("x", 16, 2, 0, function() return pos and pos.x * 65535 or 0 end)
  field("y", 18, 2, 0, function() return pos and pos.y * 65535 or 0 end)
  field("flags", 20, 1, 0, function()
    local f = 0
    if UnitAffectingCombat("player") then f = f + 1 end
    if UnitIsDeadOrGhost("player") then f = f + 2 end
    if IsResting() then f = f + 4 end
    if IsMounted() then f = f + 8 end
    return f
  end)
  field("health", 21, 1, 0, function() return pct(UnitHealth("player"), UnitHealthMax("player")) end)
  field("power", 22, 1, 0, function() return pct(UnitPower("player"), UnitPowerMax("player")) end)
  field("target", 23, 1, 255, function() return UnitExists("target") and pct(UnitHealth("target"), UnitHealthMax("target")) or 255 end)
  for i = 1, 12 do
    field("cd", 23 + i, 1, 255, function() local b = _G["MultiBarBottomLeftButton" .. i]; return cooldown(b and b.action) end)
  end
  for i = 1, 6 do
    field("cd", 35 + i, 1, 255, function() local b = _G["MultiBarBottomRightButton" .. i]; return cooldown(b and b.action) end)
  end
  field("class", 42, 1, 0, function() local _, _, id = UnitClass("player"); return id or 0 end)
  field("faction", 43, 1, 2, function()
    local f = UnitFactionGroup("player")
    return f == "Horde" and 0 or f == "Alliance" and 1 or 2
  end)
  -- Bytes to 6-bit symbols.
  local syms, acc, nbits = {}, 0, 0
  for i = 0, DATA - 1 do
    acc = acc * 256 + bytes[i]
    nbits = nbits + 8
    while nbits >= 6 do
      nbits = nbits - 6
      local d = 2 ^ nbits
      syms[#syms + 1] = math.floor(acc / d)
      acc = acc % d
    end
  end
  local sum = 0
  for i = 1, SYMBOLS do sum = (sum + (syms[i] or 0) * i) % 64 end
  seq = (seq + 1) % 64
  cells[1]:SetColorTexture(1, 0, 1, 1)
  cells[2]:SetColorTexture(0, 1, 1, 1)
  sym(3, seq)
  for i = 1, SYMBOLS do sym(3 + i, syms[i] or 0) end
  sym(SYMBOLS + 4, sum)
end

local elapsed = 0
strip:SetScript("OnUpdate", function(_, dt)
  elapsed = elapsed + dt
  if elapsed < 0.2 then return end
  elapsed = 0
  local ok, err = pcall(encode)
  if not ok and not strip.warned then
    strip.warned = true
    print("|cffffd100PULPIT|r strip: " .. tostring(err))
  end
end)
strip:RegisterEvent("DISPLAY_SIZE_CHANGED")
strip:RegisterEvent("UI_SCALE_CHANGED")
strip:SetScript("OnEvent", layout)

SLASH_PULPITSTRIP1 = "/pulpitstrip"
SlashCmdList.PULPITSTRIP = function(msg)
  if msg == "off" then strip:Hide() else strip:Show() end
  local list = {}
  for k in pairs(blocked) do list[#list + 1] = k end
  print("|cffffd100PULPIT|r strip: " .. (strip:IsShown() and "on" or "off") .. (#list > 0 and (", blocked by the game: " .. table.concat(list, ", ")) or ""))
end
