#!/bin/bash

curl -i -k -w "@curl-format.txt" -X POST -H "Content-Type: application/json" --data @login-request.json https://10.10.112.20:443/api/identity/auth/login