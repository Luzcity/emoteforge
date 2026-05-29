-- EmoteForge shared player logic (generated)
local currentProp = nil

local function clearProp()
  if currentProp and DoesEntityExist(currentProp) then
    DeleteEntity(currentProp)
  end
  currentProp = nil
end

local function attachProp(ped, prop)
  if not prop then return end
  local model = joaat(prop.model)
  RequestModel(model)
  local t = GetGameTimer()
  while not HasModelLoaded(model) do
    Wait(10)
    if GetGameTimer() - t > 5000 then return end
  end
  local c = GetEntityCoords(ped)
  local obj = CreateObject(model, c.x, c.y, c.z, true, true, false)
  AttachEntityToEntity(
    obj, ped, GetPedBoneIndex(ped, prop.bone),
    prop.offset[1] + 0.0, prop.offset[2] + 0.0, prop.offset[3] + 0.0,
    prop.rot[1] + 0.0, prop.rot[2] + 0.0, prop.rot[3] + 0.0,
    true, true, false, true, 1, true
  )
  SetModelAsNoLongerNeeded(model)
  currentProp = obj
end

local function animFlag(emote, clip)
  local f = 0
  if emote.loop then f = f | 1 end
  if emote.upperBodyOnly then f = f | 16 end
  for _, name in ipairs(clip.flags or {}) do
    if name == 'AF_LOOPING' then f = f | 1
    elseif name == 'AF_HOLD_LAST_FRAME' then f = f | 2
    elseif name == 'AF_UPPERBODY' then f = f | 16
    elseif name == 'AF_SECONDARY' then f = f | 32
    elseif name == 'AF_TAG_SYNC_OUT' then f = f | 64 end
  end
  if f == 0 and emote.movementType == 'walkable' then f = 51 end
  return f
end

local function ensureDict(dict)
  RequestAnimDict(dict)
  local t = GetGameTimer()
  while not HasAnimDictLoaded(dict) do
    Wait(10)
    if GetGameTimer() - t > 5000 then return false end
  end
  return true
end

local function playClip(ped, emote, clip)
  if not ensureDict(clip.dict) then return false end
  local flag = animFlag(emote, clip)
  TaskPlayAnim(
    ped, clip.dict, clip.clip,
    (clip.blendIn or 1.0) + 0.0, (clip.blendOut or 1.0) + 0.0,
    clip.duration or -1, flag, (clip.playbackRate or 1.0) + 0.0,
    false, false, false
  )
  return true
end

local function stopEmote()
  ClearPedTasks(PlayerPedId())
  clearProp()
end

local function playEmoteObj(emote)
  if not emote or not emote.clips then return end
  local ped = PlayerPedId()
  CreateThread(function()
    clearProp()
    attachProp(ped, emote.prop)
    if emote.facial and ensureDict(emote.facial.dict) then
      TaskPlayAnim(ped, emote.facial.dict, emote.facial.clip, 2.0, 2.0, -1, 32, 0.0, false, false, false)
    end
    for i, clip in ipairs(emote.clips) do
      if not playClip(ped, emote, clip) then break end
      local isLast = (i == #emote.clips)
      if emote.loop and isLast then break end
      local dur = clip.duration or -1
      if dur and dur > 0 then
        Wait(dur)
      else
        local rate = (clip.playbackRate and clip.playbackRate > 0) and clip.playbackRate or 1.0
        local len = GetAnimDuration(clip.dict, clip.clip)
        Wait(math.floor((len * 1000.0) / rate))
      end
    end
  end)
end

-- preview bridge client: play emote pushed from the desktop app
RegisterNetEvent('emoteforge_bridge:play')
AddEventHandler('emoteforge_bridge:play', function(emote)
  playEmoteObj(emote)
end)

RegisterNetEvent('emoteforge_bridge:stop')
AddEventHandler('emoteforge_bridge:stop', function()
  stopEmote()
end)
