#!/bin/bash

curl -i -k -w "@curl-format.txt" -X GET -H "@auth-header.txt" https://10.10.112.20/api/foundation/v1/client/client/1
curl -i -k -w "@curl-format.txt" -X GET -H "@auth-header.txt" -G --data-urlencode 'q={"anexa_cons_id":"33606","categoria":"10"}' -d 'order=luna_calc' https://10.10.112.20/api/foundation/v1/client/evacuator_calc/
curl -i -k -w "@curl-format.txt" -X GET -H "@auth-header.txt" -G --data-urlencode 'q={"anexa_cons_id":"33606","categoria":"10"}' -d 'offset=20' -d 'limit=10' -d 'order=luna_calc' https://10.10.112.20/api/foundation/v1/client/evacuator_calc/