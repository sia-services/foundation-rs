#!/bin/bash

curl -i -k -w "@curl-format.txt" -X GET -H "@auth-header.txt" https://10.10.112.20/mgmt/foundation/schemas/
curl -i -k -w "@curl-format.txt" -X GET -H "@auth-header.txt" https://10.10.112.20/mgmt/foundation/schemas/client
curl -i -k -w "@curl-format.txt" -X GET -H "@auth-header.txt" https://10.10.112.20/mgmt/foundation/schemas/client/evacuator_calc
curl -i -k -w "@curl-format.txt" -X GET -H "@auth-header.txt" https://10.10.112.20/mgmt/foundation/schemas/client/client