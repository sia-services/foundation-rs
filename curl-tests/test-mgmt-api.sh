#!/bin/bash

curl -i -k -w "@curl-format.txt" -X GET -H "@auth-header.txt" https://localhost:8444/mgmt/schemas/
curl -i -k -w "@curl-format.txt" -X GET -H "@auth-header.txt" https://localhost:8444/mgmt/schemas/client
curl -i -k -w "@curl-format.txt" -X GET -H "@auth-header.txt" https://localhost:8444/mgmt/schemas/client/evacuator_calc
curl -i -k -w "@curl-format.txt" -X GET -H "@auth-header.txt" https://localhost:8444/mgmt/schemas/client/client