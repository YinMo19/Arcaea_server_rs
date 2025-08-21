#!/bin/bash

MAP_ID=${1:-test}

curl -X GET "http://localhost:8090/yinmo/30/world/map/me/${MAP_ID}" \
     -H "Authorization: Bearer WUmp21svCqUa9BDa8NLo0r+494AAtn2xhH99IJskfEs="
