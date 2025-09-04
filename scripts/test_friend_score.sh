#!/bin/bash

# Get auth token first
TOKEN=$(curl -s -X POST "http://localhost:8090/auth/login" \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -H "Authorization: Basic Y2lhbGxvOjBkMDAwNzIx" \
     -d "grant_type=client_credentials" | jq -r '.access_token')

# Test friend score query
curl -X GET "http://localhost:8090/yinmo/30/score/song/friend?song_id=testify&difficulty=2" \
     -H "Authorization: Bearer $TOKEN"
