# An old school server for the game Honfoglaló

This project is still a WIP, but a playable version is coming soon!

First, I want to finish todo.md and then publish a playable version.

Reproduce:

Execute this twice:
curl -H 'content-type: application/octet-stream' -H 'accept: */*' -H 'user-agent: Ruffle/0.1.0' -H 'host: localhost:8080' -X POST 'http://localhost:8080/game?CID=1&CH=L&MN=1&TRY=1' -d '<L CID="1" MN="1"  />\x0d\x0a<LISTEN READY="1" />\x0d\x0a'

Execute this once:
curl -H 'content-type: application/octet-stream' -H 'accept: */*' -H 'user-agent: Ruffle/0.1.0' -H 'host: localhost:8080' -X POST 'http://localhost:8080/game?CID=1&CH=C&MN=1&TRY=1' -d '<C CID="1" MN="1"  />\x0d\x0a<CHANGEWAITHALL WH="GAME" />\x0d\x0a'
