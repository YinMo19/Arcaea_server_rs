#!/bin/bash

# Test world mode token generation
echo "Testing world mode token generation..."

# Test basic world token request
echo "1. Testing basic world token request:"
curl -X GET "http://localhost:8090/yinmo/30/score/token/world?song_id=grievouslady&difficulty=2&stamina_multiply=1&fragment_multiply=100" \
     -H "Authorization: Bearer WUmp21svCqUa9BDa8NLo0r+494AAtn2xhH99IJskfEs=" \
     -w "\nStatus: %{http_code}\n" \
     -s

echo -e "\n2. Testing world token with all parameters:"
curl -X GET "http://localhost:8090/yinmo/30/score/token/world?song_id=grievouslady&difficulty=2&stamina_multiply=1&fragment_multiply=100&prog_boost_multiply=0&beyond_boost_gauge_use=0&skill_id=skill_vita&is_skill_sealed=false" \
     -H "Authorization: Bearer WUmp21svCqUa9BDa8NLo0r+494AAtn2xhH99IJskfEs=" \
     -w "\nStatus: %{http_code}\n" \
     -s

echo -e "\n3. Testing without authorization (should fail):"
curl -X GET "http://localhost:8090/yinmo/30/score/token/world?song_id=grievouslady&difficulty=2" \
     -w "\nStatus: %{http_code}\n" \
     -s

echo -e "\n4. Testing invalid song parameters:"
curl -X GET "http://localhost:8090/yinmo/30/score/token/world?song_id=&difficulty=-1" \
     -H "Authorization: Bearer WUmp21svCqUa9BDa8NLo0r+494AAtn2xhH99IJskfEs=" \
     -w "\nStatus: %{http_code}\n" \
     -s

echo -e "\nTesting completed."
