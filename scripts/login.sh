curl -X POST "http://localhost:8090/auth/login" \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -H "Authorization: Basic Y2lhbGxvOjBkMDAwNzIx" \
     -d "grant_type=client_credentials"
