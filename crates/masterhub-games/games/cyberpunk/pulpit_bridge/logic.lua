-- PULPIT bridge logic: stats + commands. Reload from outside with the "reload" command.
local try = PB.try

function PB.stats()
  local p = Game.GetPlayer()
  if not p then return { ingame = false } end
  local id = p:GetEntityID()
  local sp, ss = Game.GetStatPoolsSystem(), Game.GetStatsSystem()
  local s = { ingame = true, god = PB.god or false, stamina_inf = PB.stamina or false }
  s.hp = try(function() return sp:GetStatPoolValue(id, gamedataStatPoolType.Health, true) end)
  s.ram = try(function() return sp:GetStatPoolValue(id, gamedataStatPoolType.Memory, false) end)
  s.ram_max = try(function() return ss:GetStatValue(id, gamedataStatType.Memory) end)
  s.eddies = try(function() return Game.GetTransactionSystem():GetItemQuantity(p, MarketSystem.Money()) end)
  s.level = try(function() return ss:GetStatValue(id, gamedataStatType.Level) end)
  s.cred = try(function() return ss:GetStatValue(id, gamedataStatType.StreetCred) end)
  s.district = try(function()
    local bb = Game.GetBlackboardSystem():Get(GetAllBlackboardDefs().UI_Map)
    local k = bb:GetString(GetAllBlackboardDefs().UI_Map.currentLocation)
    return GetLocalizedText(k)
  end)
  s.wanted = try(function()
    return tonumber(EnumInt(Game.GetScriptableSystemsContainer():Get("PreventionSystem"):GetHeatStage()))
  end)
  s.speed = try(function() local v = Game.GetMountedVehicle(p) return v and v:GetCurrentSpeed() * 3.6 or nil end)
  s.weapon = try(function()
    local w = ScriptedPuppet.GetWeaponRight(p)
    if not w then return nil end
    local ts = Game.GetTransactionSystem()
    local r = { mag = WeaponObject.GetMagazineAmmoCount(w),
                reserve = ts:GetItemQuantity(p, WeaponObject.GetAmmoType(w)),
                name = GetLocalizedTextByKey(TweakDBInterface.GetItemRecord(w:GetItemID():GetTDBID()):DisplayName()) }
    local l = ScriptedPuppet.GetWeaponLeft(p)
    if l then r.mag_left = WeaponObject.GetMagazineAmmoCount(l) end
    return r
  end)
  s.combat = try(function() return p:IsInCombat() end) or false
  s.sandy = try(function() return sp:GetStatPoolValue(id, gamedataStatPoolType.SandevistanCharge, true) end)
  s.berserk = try(function() return sp:GetStatPoolValue(id, gamedataStatPoolType.BerserkCharge, true) end)
  s.heal_charge = try(function() return sp:GetStatPoolValue(id, gamedataStatPoolType.HealingItemsCharges, true) end)
  s.time = try(function() local g = Game.GetTimeSystem():GetGameTime() return string.format("%02d:%02d", g:Hours(), g:Minutes()) end)
  return s
end


-- commands ---------------------------------------------------------------
local C = {}
PB.C = C
local function P() local p = Game.GetPlayer() if not p then error("not in game") end return p end
local function dev() return PlayerDevelopmentSystem.GetData(P()) end
local function fill(pool) Game.GetStatPoolsSystem():RequestSettingStatPoolMaxValue(P():GetEntityID(), pool, P()) end

function C.eddies(arg) local n = math.floor(tonumber(arg) or 10000) Game.AddToInventory("Items.money", n) return n end
function C.give(arg)   -- "Items.Preset_Overture_Kerry 1"
  local id, n = arg:match("^(%S+)%s*(%d*)$")
  Game.AddToInventory(id, tonumber(n) or 1) return id
end
local function levelUp(kind)
  local d = dev()
  local need = d:GetRemainingExpForLevelUp(kind)
  d:AddExperience(need, kind, telemetryLevelGainReason.Gameplay)
  return need
end
function C.level() return levelUp(gamedataProficiencyType.Level) end
function C.cred()  return levelUp(gamedataProficiencyType.StreetCred) end
local function point(kind) dev():AddDevelopmentPoints(1, kind) return dev():GetDevPoints(kind) end
function C.perk()  return point(gamedataDevelopmentPointType.Primary) end
function C.attr()  return point(gamedataDevelopmentPointType.Attribute) end
function C.relic() return point(gamedataDevelopmentPointType.Espionage) end
-- Max the character: level 60, street cred 50, every attribute 20, enough perk points for every
-- perk (their total cost, 241 on patch 2.5, minus what levelling gives) and 20 relic points.
function C.maxout()
  local d = dev()
  local function upTo(kind, cap)
    local n = 0
    while d:GetProficiencyLevel(kind) < cap and n < 100 do
      d:AddExperience(d:GetRemainingExpForLevelUp(kind), kind, telemetryLevelGainReason.Gameplay)
      n = n + 1
    end
    return n
  end
  local lv = upTo(gamedataProficiencyType.Level, d:GetProficiencyAbsoluteMaxLevel(gamedataProficiencyType.Level))
  local cr = upTo(gamedataProficiencyType.StreetCred, d:GetProficiencyAbsoluteMaxLevel(gamedataProficiencyType.StreetCred))
  for _, a in ipairs({ "Strength", "Reflexes", "TechnicalAbility", "Intelligence", "Cool" }) do
    d:SetAttribute(gamedataStatType[a], 20)
  end
  local cost = 0
  for _, r in ipairs(TweakDB:GetRecords("gamedataNewPerk_Record") or {}) do
    local ok, n = pcall(function() return r:GetLevelsCount() end)
    if ok and n then cost = cost + n end
  end
  local extra = math.max(0, cost - lv)
  d:AddDevelopmentPoints(extra, gamedataDevelopmentPointType.Primary)
  d:AddDevelopmentPoints(20, gamedataDevelopmentPointType.Espionage)
  return string.format("levels +%d, cred +%d, attributes 20, perk points +%d (now %d), relic +20",
    lv, cr, extra, d:GetDevPoints(gamedataDevelopmentPointType.Primary))
end

function C.heal() fill(gamedataStatPoolType.Health) return "ok" end
function C.ram()  fill(gamedataStatPoolType.Memory) return "ok" end
local AMMO = { "Ammo.HandgunAmmo", "Ammo.RifleAmmo", "Ammo.ShotgunAmmo", "Ammo.SniperRifleAmmo" }
function C.ammo()
  local ts, p = Game.GetTransactionSystem(), P()
  for _, a in ipairs(AMMO) do
    local have = ts:GetItemQuantity(p, ItemID.FromTDBID(TweakDBID.new(a)))
    if have < 500 then Game.AddToInventory(a, 500 - have) end
  end
  return "ok"
end
function C.wanted()
  local ps = Game.GetScriptableSystemsContainer():Get("PreventionSystem")
  local errs = {}
  for _, f in ipairs({
    function() ps:ChangeHeatStage(EPreventionHeatStage.Heat_0, "PULPIT") end,
    function() PreventionSystem.ClearHeatStage(ps) end,
    function() ps:ClearHeat() end }) do
    local ok, e = pcall(f) if ok then return tostring(ps:GetHeatStage()) end errs[#errs + 1] = tostring(e)
  end
  error(table.concat(errs, " / "))
end
function C.god()
  local g, id = Game.GetGodModeSystem(), P():GetEntityID()
  PB.god = not PB.god
  if PB.god then g:AddGodMode(id, gameGodModeType.Invulnerable, CName.new("PULPIT"))
  else g:RemoveGodMode(id, gameGodModeType.Invulnerable, CName.new("PULPIT")) end
  return PB.god
end
function C.stamina() PB.stamina = not PB.stamina return PB.stamina end
local WEATHER = { "24h_weather_sunny", "24h_weather_light_clouds", "24h_weather_cloudy", "24h_weather_heavy_clouds",
  "24h_weather_fog", "24h_weather_rain", "24h_weather_toxic_rain", "24h_weather_pollution", "24h_weather_sandstorm" }
function C.weather(arg)
  local ws = Game.GetWeatherSystem()
  local now = ws:GetWeatherState().name.value
  local nxt = arg ~= "" and arg or WEATHER[1]
  if arg == "" then for i, w in ipairs(WEATHER) do if w == now then nxt = WEATHER[i % #WEATHER + 1] end end end
  ws:SetWeather(nxt, 10, 9)
  return nxt
end
function C.weather_reset() Game.GetWeatherSystem():ResetWeather(true) return "ok" end
function C.waypoint_info()
  local out = {}
  for _, m in ipairs(Game.GetMappinSystem():GetMappins(gamemappinsMappinTargetType.World)) do
    local v = tostring(m:GetVariant()):gsub("gamedataMappinVariant : ", "")
    if v:find("Custom") or m:IsPlayerTracked() then
      local p = m:GetWorldPosition()
      out[#out + 1] = string.format("%s tracked=%s %.1f %.1f %.1f", v, tostring(m:IsPlayerTracked()), p.x, p.y, p.z)
    end
  end
  return table.concat(out, " ; ")
end
-- The player's own map pin first, else the tracked quest marker.
local function customWaypoint()
  local tracked
  for _, m in ipairs(Game.GetMappinSystem():GetMappins(gamemappinsMappinTargetType.World)) do
    if m:GetVariant() == gamedataMappinVariant.CustomPositionVariant then return m:GetWorldPosition(), "pin" end
    if not tracked and m:IsPlayerTracked() then tracked = m:GetWorldPosition() end
  end
  if tracked then return tracked, "quest" end
end
function C.waypoint()
  local pos, kind = customWaypoint()
  if not pos then error("no waypoint on the map") end
  local p = P()
  PB.back = p:GetWorldPosition()
  local z = pos.z
  if math.abs(z) < 0.01 then z = PB.back.z + 50 end  -- no height on the pin: drop in from above
  Game.GetTeleportationFacility():Teleport(p, ToVector4 { x = pos.x, y = pos.y, z = z + 1, w = 1 }, p:GetWorldOrientation():ToEulerAngles())
  return string.format("%s %.0f %.0f %.0f", kind, pos.x, pos.y, z)
end
function C.back()
  if not PB.back then error("nothing to go back to") end
  local p = P()
  Game.GetTeleportationFacility():Teleport(p, PB.back, p:GetWorldOrientation():ToEulerAngles())
  return "ok"
end

-- Item list for the spawn panel: items.json, built a few hundred records per frame (the CET
-- sandbox has no coroutines, so the dump is a small state machine stepped from tick).
local function esc(x) return PB.enc(x) end
local function itemRow(r)
  local name = GetLocalizedTextByKey(r:DisplayName())
  if name == nil or name == "" then return nil end
  local iconic, evo, area = false, "", ""
  pcall(function() iconic = r:TagsContains(CName.new("IconicWeapon")) end)
  pcall(function() evo = tostring(r:Evolution():Type()):match(": ([%w_]+)") or "" end)
  pcall(function() area = tostring(r:EquipArea():Type()):match(": ([%w_]+)") or "" end)
  return "{" .. '"evo":' .. esc(evo) .. ',"area":' .. esc(area) .. ',"id":' .. esc(TDBID.ToStringDEBUG(r:GetID())) .. ',"name":' .. esc(name)
    .. ',"type":' .. esc(tostring(r:ItemType():Type()):match(": ([%w_]+)") or "")
    .. ',"quality":' .. esc(tostring(r:Quality():Type()):match(": (%w+)") or "")
    .. ',"cat":' .. esc(tostring(r:ItemCategory():Type()):match(": (%w+)") or "")
    .. ',"iconic":' .. tostring(iconic) .. "}"
end
local function carRow(r)
  local id = TDBID.ToStringDEBUG(r:GetID())
  if not id:find("_player") then return nil end
  local name = GetLocalizedTextByKey(r:DisplayName())
  if name == nil or name == "" then return nil end
  local make = ""
  pcall(function() make = GetLocalizedTextByKey(r:Manufacturer():DisplayName()) or "" end)
  return '{"id":' .. esc(id) .. ',"name":' .. esc(name) .. ',"make":' .. esc(make) .. "}"
end
local function dumpStep()
  local d = PB.dump
  local stop = math.min(d.i + 299, #d.recs)
  for i = d.i, stop do
    local ok, row = pcall(itemRow, d.recs[i])
    if ok and row then d.out[#d.out + 1] = row end
  end
  d.i = stop + 1
  if d.i <= #d.recs then return end
  local cars = {}
  for _, r in ipairs(TweakDB:GetRecords("gamedataVehicle_Record")) do
    local ok, row = pcall(carRow, r)
    if ok and row then cars[#cars + 1] = row end
  end
  PB.write("items.json", '{"items":[' .. table.concat(d.out, ",") .. '],"vehicles":[' .. table.concat(cars, ",") .. "]}")
  PB.dump_done = #d.out .. " items, " .. #cars .. " vehicles"
  PB.dump = nil
end
function C.dump_items()
  local recs = {}
  for _, t in ipairs({ "gamedataItem_Record", "gamedataWeaponItem_Record", "gamedataClothing_Record",
                       "gamedataConsumableItem_Record", "gamedataGrenade_Record" }) do
    for _, r in ipairs(TweakDB:GetRecords(t) or {}) do recs[#recs + 1] = r end
  end
  PB.dump = { recs = recs, i = 1, out = {} }
  PB.dump_done = nil
  return "started"
end
function C.dump_status() return PB.dump_done or "running" end
-- Loadouts ---------------------------------------------------------------
local AREAS = { "Weapon", "Head", "Face", "OuterChest", "InnerChest", "Legs", "Feet", "Outfit",
  "FrontalCortexCW", "SystemReplacementCW", "ArmsCW", "HandsCW", "EyesCW", "LegsCW", "NervousSystemCW",
  "CardiovascularSystemCW", "IntegumentarySystemCW", "MusculoskeletalSystemCW", "ImmuneSystemCW" }
local function playerData() return Game.GetScriptableSystemsContainer():Get("EquipmentSystem"):GetPlayerData(P()) end

-- What the player has equipped now: [{area, slot, id, name}], as JSON text.
function C.equipped()
  local pd, rows = playerData(), {}
  for _, a in ipairs(AREAS) do
    pcall(function()
      local area = gamedataEquipmentArea[a]
      for i = 0, pd:GetNumberOfSlots(area, false) - 1 do
        local it = pd:GetItemInEquipSlot(area, i)
        if ItemID.IsValid(it) then
          local tdb = ItemID.GetTDBID(it)
          local name = ""
          pcall(function() name = GetLocalizedTextByKey(TweakDBInterface.GetItemRecord(tdb):DisplayName()) end)
          rows[#rows + 1] = '{"area":' .. PB.enc(a) .. ',"slot":' .. i .. ',"id":' .. PB.enc(TDBID.ToStringDEBUG(tdb)) .. ',"name":' .. PB.enc(name) .. "}"
        end
      end
    end)
  end
  return "[" .. table.concat(rows, ",") .. "]"
end

-- An owned copy of item `id` (a TweakDB id string), if any.
local function owned(id)
  local ok, list = Game.GetTransactionSystem():GetItemList(P())
  if not ok or not list then return nil end
  for _, it in ipairs(list) do
    local iid = it:GetID()
    if TDBID.ToStringDEBUG(ItemID.GetTDBID(iid)) == id then return iid end
  end
end

-- "equip;id qty area slot|id qty area slot|…" (equip = 1/0; area/slot may be "-").
-- Gives what is missing, then (equip=1) puts each item with an area into its slot.
function C.apply(arg)
  local equip, rest = arg:match("^(%d);(.*)$")
  local given, equipped, failed = 0, 0, {}
  local es = Game.GetScriptableSystemsContainer():Get("EquipmentSystem")
  for entry in (rest or ""):gmatch("[^|]+") do
    local id, qty, area, slot = entry:match("^(%S+)%s+(%d+)%s+(%S+)%s+(%S+)$")
    if id then
      local ok, e = pcall(function()
        local have = owned(id)
        if not have then
          Game.AddToInventory(id, tonumber(qty) or 1)
          given = given + 1
          have = owned(id)
        end
        if equip == "1" and have and area ~= "-" then
          local req = EquipRequest.new()
          req.itemID = have
          req.owner = P()
          if slot ~= "-" then req.slotIndex = tonumber(slot) end
          es:QueueRequest(req)
          equipped = equipped + 1
        end
      end)
      if not ok then failed[#failed + 1] = id .. ": " .. tostring(e):sub(1, 60) end
    end
  end
  local r = string.format("given %d, equipped %d", given, equipped)
  if #failed > 0 then r = r .. ", failed " .. #failed .. " (" .. table.concat(failed, "; ") .. ")" end
  return r
end

-- Perk trees ------------------------------------------------------------
-- Bought perks as JSON text: [{"id","level","name"}].
function C.perks_get()
  local d, rows = dev(), {}
  for _, r in ipairs(TweakDB:GetRecords("gamedataNewPerk_Record") or {}) do
    pcall(function()
      local lv = d:IsNewPerkBought(r:Type())
      if lv and lv > 0 then
        local name = ""
        pcall(function() name = GetLocalizedText(r:Loc_name_key()) or "" end)
        rows[#rows + 1] = '{"id":' .. PB.enc(TDBID.ToStringDEBUG(r:GetID())) .. ',"level":' .. lv .. ',"name":' .. PB.enc(name) .. "}"
      end
    end)
  end
  return "[" .. table.concat(rows, ",") .. "]"
end

-- "id level|id level|…": refund every perk, then buy the saved ones. Perks need their
-- parents first, so buying goes in passes until nothing more can be bought.
function C.perks_apply(arg)
  local d = dev()
  local want = {}
  for id, lv in arg:gmatch("([^%s|]+)%s+(%d+)") do
    local r = TweakDBInterface.GetNewPerkRecord(TweakDBID.new(id))
    if r then want[#want + 1] = { t = r:Type(), lv = tonumber(lv), id = id } end
  end
  d:ResetNewPerks()
  -- A reset leaves perks locked (unlocks normally follow attribute changes): unlock what the
  -- set needs, and with every attribute at 20 the whole tree, as the game would.
  local maxed = true
  for _, a in ipairs({ "Strength", "Reflexes", "TechnicalAbility", "Intelligence", "Cool" }) do
    if d:GetAttributeValue(gamedataStatType[a]) < 20 then maxed = false end
  end
  if maxed then
    for _, r in ipairs(TweakDB:GetRecords("gamedataNewPerk_Record") or {}) do pcall(function() d:UnlockNewPerk(r:Type()) end) end
  end
  for _, w in ipairs(want) do pcall(function() d:UnlockNewPerk(w.t) end) end
  local bought, total = 0, 0
  for _, w in ipairs(want) do total = total + w.lv end
  for _ = 1, 12 do
    local progress = false
    for _, w in ipairs(want) do
      local have = d:IsNewPerkBought(w.t) or 0
      while have < w.lv do
        local ok = pcall(function() return d:BuyNewPerk(w.t) end)
        local now = d:IsNewPerkBought(w.t) or 0
        if not ok or now <= have then break end
        have = now
        bought = bought + 1
        progress = true
      end
    end
    if not progress then break end
  end
  local missing = {}
  for _, w in ipairs(want) do
    if (d:IsNewPerkBought(w.t) or 0) < w.lv then missing[#missing + 1] = w.id end
  end
  local r = string.format("perks %d/%d", bought, total)
  if #missing > 0 then r = r .. ", missing " .. table.concat(missing, " ") end
  return r
end

-- Icons: every UIIcon record (atlas hash, part), each item's icon hints and each perk's icon,
-- to icons.json for games/cyberpunk/icons/build.py (which cuts the pictures from the atlases).
local function cname(n)
  local ok, v = pcall(function() return n.value end)
  if ok and v and v ~= "None" then return v end
  return ""
end
function C.dump_icons()
  local rows = {}
  for _, r in ipairs(TweakDB:GetRecords("gamedataUIIcon_Record") or {}) do
    pcall(function()
      local rid = TDBID.ToStringDEBUG(r:GetID())
      local h = tostring(TweakDB:GetFlat(TweakDBID.new(rid .. ".atlasResourcePath")).hash):gsub("ULL", "")
      rows[#rows + 1] = "[" .. PB.enc(TDBID.ToStringDEBUG(r:GetID())) .. "," .. PB.enc(h) .. "," .. PB.enc(cname(r:AtlasPartName())) .. "]"
    end)
  end
  local items = {}
  for _, t in ipairs({ "gamedataItem_Record", "gamedataWeaponItem_Record", "gamedataClothing_Record",
                       "gamedataConsumableItem_Record", "gamedataGrenade_Record" }) do
    for _, r in ipairs(TweakDB:GetRecords(t) or {}) do
      pcall(function()
        local id = TDBID.ToStringDEBUG(r:GetID())
        local icon = TweakDB:GetFlat(TweakDBID.new(id .. ".iconPath")) or ""
        local app = cname(TweakDB:GetFlat(TweakDBID.new(id .. ".appearanceName")))
        local rec = ""
        pcall(function() rec = TDBID.ToStringDEBUG(r:Icon():GetID()) end)
        -- Weapons: icon = weapon_<appearanceResourceName>_None__<first visual tag>.
        local res, tag = "", ""
        pcall(function() res = cname(TweakDB:GetFlat(TweakDBID.new(id .. ".appearanceResourceName"))) end)
        pcall(function() local t = r:VisualTags() if t and t[1] then tag = t[1].value end end)
        if icon ~= "" or app ~= "" or res ~= "" or (rec ~= "" and rec ~= "UIIcon.ItemIcon") then
          items[#items + 1] = "[" .. PB.enc(id) .. "," .. PB.enc(icon) .. "," .. PB.enc(app) .. "," .. PB.enc(rec)
            .. "," .. PB.enc(res) .. "," .. PB.enc(tag) .. "]"
        end
      end)
    end
  end
  local perks = {}
  for _, r in ipairs(TweakDB:GetRecords("gamedataNewPerk_Record") or {}) do
    pcall(function()
      perks[#perks + 1] = "[" .. PB.enc(TDBID.ToStringDEBUG(r:GetID())) .. "," .. PB.enc(TDBID.ToStringDEBUG(r:PerkIcon():GetID())) .. "]"
    end)
  end
  local female = false
  pcall(function() female = Game.GetPlayer():GetResolvedGenderName().value == "Female" end)
  PB.write("icons.json", '{"female":' .. tostring(female) .. ',"uiicons":[' .. table.concat(rows, ",") .. '],"items":[' .. table.concat(items, ",") .. '],"perks":[' .. table.concat(perks, ",") .. "]}")
  return #rows .. " icons, " .. #items .. " items with hints, " .. #perks .. " perks"
end

-- Every perk for the build editor's trees: perks.json [{id, name, desc, max, attr}].
function C.dump_perks()
  local rows = {}
  for _, r in ipairs(TweakDB:GetRecords("gamedataNewPerk_Record") or {}) do
    pcall(function()
      local name, desc, attr = "", "", ""
      pcall(function() name = GetLocalizedText(r:Loc_name_key()) or "" end)
      pcall(function() desc = GetLocalizedText(r:Loc_desc_key()) or "" end)
      pcall(function() attr = tostring(r:Attribute():Type()):match(": (%w+)") or "" end)
      local slot = ""
      pcall(function() slot = TDBID.ToStringDEBUG(r:Slot():GetID()):gsub("^NewPerks%.", "") end)
      rows[#rows + 1] = '{"id":' .. PB.enc(TDBID.ToStringDEBUG(r:GetID())) .. ',"slot":' .. PB.enc(slot) .. ',"name":' .. PB.enc(name) .. ',"desc":' .. PB.enc(desc)
        .. ',"max":' .. (r:GetLevelsCount() or 1) .. ',"attr":' .. PB.enc(attr) .. "}"
    end)
  end
  PB.write("perks.json", "[" .. table.concat(rows, ",") .. "]")
  return #rows .. " perks"
end

-- Equipment slots per area (how many each area has on this character).
function C.slots()
  local pd, rows = playerData(), {}
  for _, a in ipairs(AREAS) do
    pcall(function() rows[#rows + 1] = PB.enc(a) .. ":" .. pd:GetNumberOfSlots(gamedataEquipmentArea[a], false) end)
  end
  return "{" .. table.concat(rows, ",") .. "}"
end

-- Item details for the build editor: details.json {id: {d, e:[effects], dt}}, a few hundred
-- items per frame. Effects are the tooltip texts of the item's equip/attach packages with their
-- {float_N}/{int_N} values filled in (markup kept; the page renders it).
local function fillUI(ui)
  local text = GetLocalizedText(ui:LocalizedDescription()) or ""
  if text == "" then return nil end
  pcall(function() for i, f in ipairs(ui:FloatValues()) do
    local v = (math.abs(f - math.floor(f)) < 0.001) and string.format("%d", f) or string.format("%.1f", f)
    text = text:gsub("{float_" .. (i - 1) .. "}", v) end end)
  pcall(function() for i, n in ipairs(ui:IntValues()) do text = text:gsub("{int_" .. (i - 1) .. "}", tostring(n)) end end)
  return text
end
local function effects(list)
  local out = {}
  for _, p in ipairs(list or {}) do
    pcall(function()
      local ui = p:UIData()
      if ui then local t = fillUI(ui) if t then out[#out + 1] = PB.enc(t) end end
    end)
  end
  return out
end
local function detailRow(r)
  local id = TDBID.ToStringDEBUG(r:GetID())
  local d = ""
  pcall(function() d = GetLocalizedTextByKey(r:LocalizedDescription()) or "" end)
  local e = {}
  pcall(function() for _, x in ipairs(effects(r:OnEquip())) do e[#e + 1] = x end end)
  pcall(function() for _, x in ipairs(effects(r:OnAttach())) do e[#e + 1] = x end end)
  local dt = ""
  pcall(function() for _, g in ipairs(r:StatModifierGroups()) do
    local gid = TDBID.ToStringDEBUG(g:GetID())
    local t = gid:match("Damage_Type_(%a+)$")
    if t and t ~= "Min" then dt = t end
  end end)
  if d == "" and #e == 0 and dt == "" then return nil end
  return PB.enc(id) .. ':{"d":' .. PB.enc(d) .. ',"e":[' .. table.concat(e, ",") .. '],"dt":' .. PB.enc(dt) .. "}"
end
local function detailStep()
  local d = PB.details
  local stop = math.min(d.i + 199, #d.recs)
  for i = d.i, stop do
    local ok, row = pcall(detailRow, d.recs[i])
    if ok and row then d.out[#d.out + 1] = row end
  end
  d.i = stop + 1
  if d.i <= #d.recs then return end
  -- Perks too: the description, and each level's own line (multi-level perks keep their text
  -- per level), with the numbers of the matching values-only UI record filled in.
  local function fillValues(t, ui)
    if not ui or t == "" then return t end
    pcall(function() for i, f in ipairs(ui:FloatValues()) do
      local v = (math.abs(f - math.floor(f)) < 0.001) and string.format("%d", f) or string.format("%.1f", f)
      t = t:gsub("{float_" .. (i - 1) .. "}", v) end end)
    pcall(function() for i, n in ipairs(ui:IntValues()) do t = t:gsub("{int_" .. (i - 1) .. "}", tostring(n)) end end)
    return t
  end
  for _, r in ipairs(TweakDB:GetRecords("gamedataNewPerk_Record") or {}) do
    pcall(function()
      local ui = nil
      pcall(function() ui = r:UiData() end)
      local t = fillValues(GetLocalizedText(r:Loc_desc_key()) or "", ui)
      local lv = {}
      pcall(function() for _, l in ipairs(r:Levels()) do
        local lui = nil
        pcall(function() lui = l:UiData() end)
        local lt = ""
        pcall(function() lt = GetLocalizedText(l:Loc_desc_key()) or "" end)
        lt = fillValues(fillValues(lt, lui), ui)
        lv[#lv + 1] = PB.enc(lt)
        -- The main text of some perks takes its numbers from a level's record.
        if t:find("{") then t = fillValues(t, lui) end
      end end)
      d.out[#d.out + 1] = PB.enc(TDBID.ToStringDEBUG(r:GetID())) .. ':{"d":' .. PB.enc(t) .. ',"e":[],"dt":"","lv":[' .. table.concat(lv, ",") .. "]}"
    end)
  end
  PB.write("details.json", "{" .. table.concat(d.out, ",") .. "}")
  PB.details_done = #d.out .. " details"
  PB.details = nil
end
function C.dump_details()
  local recs = {}
  for _, t in ipairs({ "gamedataItem_Record", "gamedataWeaponItem_Record", "gamedataClothing_Record",
                       "gamedataConsumableItem_Record", "gamedataGrenade_Record" }) do
    for _, r in ipairs(TweakDB:GetRecords(t) or {}) do recs[#recs + 1] = r end
  end
  PB.details = { recs = recs, i = 1, out = {} }
  PB.details_done = nil
  return "started"
end
function C.details_status() return PB.details_done or "running" end

-- Design limits at level 60 with Phantom Liberty: 81 attribute points (each attribute 3-20,
-- the base 3s included), 80 perk points, 15 relic points.
local DESIGN = { attrs = 81, perks = 80, relic = 15 }
local ATTRS = { "Strength", "Reflexes", "TechnicalAbility", "Intelligence", "Cool" }

-- "body reflexes tech intelligence cool"
function C.set_attrs(arg)
  local d, i, vals = dev(), 0, {}
  for v in arg:gmatch("%d+") do i = i + 1 vals[i] = math.max(3, math.min(20, tonumber(v))) end
  if i ~= 5 then error("need 5 values") end
  for k, a in ipairs(ATTRS) do d:SetAttribute(gamedataStatType[a], vals[k]) end
  return table.concat(vals, " ")
end

function C.attrs_get()
  local d, o = dev(), {}
  for _, a in ipairs(ATTRS) do o[#o + 1] = string.format("%d", d:GetAttributeValue(gamedataStatType[a])) end
  return table.concat(o, " ")
end

-- Relic: the first perk of a branch costs 3, every other level 1.
local function relicCost(id) return id:find("Milestone") and 3 or 1 end

-- Set the unspent points to what the design leaves after this build.
function C.design_points()
  local d = dev()
  local perks, relic, attrs = 0, 0, 0
  for _, r in ipairs(TweakDB:GetRecords("gamedataNewPerk_Record") or {}) do
    pcall(function()
      local lv = d:IsNewPerkBought(r:Type()) or 0
      if lv > 0 then
        local id = TDBID.ToStringDEBUG(r:GetID())
        if id:find("Espionage") then relic = relic + relicCost(id) * lv else perks = perks + lv end
      end
    end)
  end
  for _, a in ipairs(ATTRS) do attrs = attrs + d:GetAttributeValue(gamedataStatType[a]) end
  local function setTo(kind, want)
    local have = d:GetDevPoints(kind)
    if want ~= have then d:AddDevelopmentPoints(want - have, kind) end
    return d:GetDevPoints(kind)
  end
  local p = setTo(gamedataDevelopmentPointType.Primary, math.max(0, DESIGN.perks - perks))
  local r = setTo(gamedataDevelopmentPointType.Espionage, math.max(0, DESIGN.relic - relic))
  local at = setTo(gamedataDevelopmentPointType.Attribute, math.max(0, DESIGN.attrs - attrs))
  return string.format("perks %d spent, %d left · relic %d spent, %d left · attributes %d, %d left", perks, p, relic, r, attrs, at)
end

function C.vehicle(arg)
  Game.GetVehicleSystem():EnablePlayerVehicle(arg, true, false)
  return arg
end

-- The spawn panel writes its own command file (panel.txt, same line format) so it never races
-- PULPIT over cmd.txt; results go to panel_result.json.
PB.panel_last = PB.panel_last or nil
PB.panel_results = PB.panel_results or {}
PB.panel_t = 0
local function pollPanel()
  local f = io.open("panel.txt", "r") if not f then PB.panel_last = PB.panel_last or 0 return end
  local data = f:read("*a") f:close()
  local first = PB.panel_last == nil
  for line in data:gmatch("([^\n]*)\n") do
    local seq, cmd, arg = line:match("^(%d+)\t([%w_]+)\t?(.*)$")
    seq = tonumber(seq)
    if seq and seq > (PB.panel_last or -1) then
      PB.panel_last = seq
      if not first then
        local fn = C[cmd]
        local ok, r
        if fn then ok, r = pcall(fn, arg) else ok, r = false, "unknown command " .. cmd end
        table.insert(PB.panel_results, 1, { seq = seq, cmd = cmd, ok = ok, value = ok and r or tostring(r) })
        while #PB.panel_results > 10 do table.remove(PB.panel_results) end
        PB.write("panel_result.json", PB.enc(PB.panel_results))
      end
    end
  end
  if first then PB.panel_last = PB.panel_last or 0 end
end

function PB.tick(dt)
  PB.panel_t = PB.panel_t + dt
  if PB.panel_t >= 0.05 then PB.panel_t = 0 PB.try(pollPanel) end
  if PB.stamina then PB.try(fill, gamedataStatPoolType.Stamina) end
  if PB.details then
    local ok, e = pcall(detailStep)
    if not ok then PB.details_done = "error: " .. tostring(e) PB.details = nil end
  end
  if PB.dump then
    local ok, e = pcall(dumpStep)
    if not ok then PB.dump_done = "error: " .. tostring(e) PB.dump = nil end
  end
end
