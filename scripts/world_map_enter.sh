#!/bin/bash

MAP_ID=${1:-test}

curl -X POST "http://localhost:8090/yinmo/30/world/map/me" \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer WUmp21svCqUa9BDa8NLo0r+494AAtn2xhH99IJskfEs=" \
     -d "{\"map_id\": \"${MAP_ID}\"}"
