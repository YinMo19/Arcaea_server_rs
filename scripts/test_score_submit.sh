#!/bin/bash

# Test score submission functionality
echo "Testing score submission..."

# First, get a world token
echo "1. Getting world mode token..."
WORLD_TOKEN_RESPONSE=$(curl -s -X GET "http://localhost:8090/yinmo/30/score/token/world?song_id=grievouslady&difficulty=2&stamina_multiply=1&fragment_multiply=100" \
     -H "Authorization: Bearer WUmp21svCqUa9BDa8NLo0r+494AAtn2xhH99IJskfEs=")

echo "World token response: $WORLD_TOKEN_RESPONSE"

# Extract token from JSON response
WORLD_TOKEN=$(echo $WORLD_TOKEN_RESPONSE | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
echo "Extracted token: $WORLD_TOKEN"

if [ -z "$WORLD_TOKEN" ]; then
    echo "Failed to get world token. Exiting."
    exit 1
fi

# Now submit a score using the token
echo -e "\n2. Submitting score..."
curl -X POST "http://localhost:8090/yinmo/30/score/song" \
     -H "Authorization: Bearer WUmp21svCqUa9BDa8NLo0r+494AAtn2xhH99IJskfEs=" \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "song_token=$WORLD_TOKEN" \
     -d "song_hash=abcd1234hash" \
     -d "song_id=grievouslady" \
     -d "difficulty=2" \
     -d "score=9876543" \
     -d "shiny_perfect_count=100" \
     -d "perfect_count=800" \
     -d "near_count=50" \
     -d "miss_count=0" \
     -d "health=100" \
     -d "modifier=0" \
     -d "clear_type=2" \
     -d "beyond_gauge=0" \
     -d "submission_hash=dummy_hash_for_testing" \
     -w "\nStatus: %{http_code}\n" \
     -s

echo -e "\n3. Testing with invalid token..."
curl -X POST "http://localhost:8090/yinmo/30/score/song" \
     -H "Authorization: Bearer WUmp21svCqUa9BDa8NLo0r+494AAtn2xhH99IJskfEs=" \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "song_token=invalid_token_123" \
     -d "song_hash=abcd1234hash" \
     -d "song_id=grievouslady" \
     -d "difficulty=2" \
     -d "score=9876543" \
     -d "shiny_perfect_count=100" \
     -d "perfect_count=800" \
     -d "near_count=50" \
     -d "miss_count=0" \
     -d "health=100" \
     -d "modifier=0" \
     -d "clear_type=2" \
     -d "beyond_gauge=0" \
     -d "submission_hash=dummy_hash_for_testing" \
     -w "\nStatus: %{http_code}\n" \
     -s

echo -e "\n4. Testing with missing required fields..."
curl -X POST "http://localhost:8090/yinmo/30/score/song" \
     -H "Authorization: Bearer WUmp21svCqUa9BDa8NLo0r+494AAtn2xhH99IJskfEs=" \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "song_token=$WORLD_TOKEN" \
     -d "song_id=grievouslady" \
     -d "difficulty=2" \
     -d "score=9876543" \
     -w "\nStatus: %{http_code}\n" \
     -s

echo -e "\nScore submission testing completed."
