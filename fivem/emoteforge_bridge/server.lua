-- preview bridge server: receive emote JSON over HTTP and forward to clients
SetHttpHandler(function(req, res)
  if req.method == 'POST' and req.path == '/preview' then
    req.setDataHandler(function(body)
      local ok, emote = pcall(json.decode, body)
      if ok and emote then
        TriggerClientEvent('emoteforge_bridge:play', -1, emote)
        res.writeHead(200, { ['Content-Type'] = 'application/json' })
        res.send('{"ok":true}')
      else
        res.writeHead(400, { ['Content-Type'] = 'application/json' })
        res.send('{"ok":false,"error":"invalid json"}')
      end
    end)
  elseif req.method == 'POST' and req.path == '/stop' then
    TriggerClientEvent('emoteforge_bridge:stop', -1)
    res.writeHead(200, { ['Content-Type'] = 'application/json' })
    res.send('{"ok":true}')
  else
    res.writeHead(404, { ['Content-Type'] = 'application/json' })
    res.send('{"ok":false,"error":"not found"}')
  end
end)
