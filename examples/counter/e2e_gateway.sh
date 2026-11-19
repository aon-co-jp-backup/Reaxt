#!/usr/bin/env bash
# 実機の通し試験: ゲートウェイ(open-cosmo) の後ろに Reaxt のサーバー2つ(serve / serve_greeter)を置き、
# 結合・変数・フラグメント・変更・エラーを本物のHTTPで確かめる。
# Real end-to-end check: two real Reaxt servers behind the open-cosmo gateway.
# 使い方: examples/counter/e2e_gateway.sh   (事前に cargo build で serve / serve_greeter / open-runo-gateway を作る)
set -u
REAXT="${REAXT:-$(cd "$(dirname "$0")/../.." && pwd)}"
COSMO="${COSMO:-$REAXT/../open-cosmo}"
EXE=""; [ -f "$REAXT/target/debug/serve.exe" ] && EXE=".exe"
"$REAXT/target/debug/serve$EXE" "$REAXT/examples/counter/pkg" 8792 >/dev/null 2>&1 & P1=$!
"$REAXT/target/debug/serve_greeter$EXE" 8793 >/dev/null 2>&1 & P2=$!
OPEN_RUNO_SUBGRAPHS="counter=http://127.0.0.1:8792/_reaxt/graphql,greeter=http://127.0.0.1:8793/_reaxt/graphql" \
OPEN_RUNO_BIND_ADDR=127.0.0.1:8099 "$COSMO/target/debug/open-runo-gateway$EXE" >/dev/null 2>&1 & P3=$!
trap 'kill $P1 $P2 $P3 2>/dev/null' EXIT
sleep 4
fail=0
q() { curl -s -XPOST localhost:8099/graphql -H 'content-type: application/json' -H 'x-reaxt: 1' -d "$1"; }
check() { # 名前, 応答, 含むべき文字列, 含んではいけない文字列(任意)
  if echo "$2" | grep -qF -- "$3" && { [ -z "${4:-}" ] || ! echo "$2" | grep -qF -- "$4"; }; then echo "ok   $1"; else echo "FAIL $1: $2"; fail=$((fail+1)); fi
}
r=$(q '{"query":"{ demo_v1_Greeter_GetVisits }"}'); check "single-subgraph read" "$r" '"demo_v1_Greeter_GetVisits":0'
r=$(q '{"query":"{ a: demo_v1_Greeter_GetVisits b: demo_v1_Counter_GetCount }"}'); check "split across two servers + alias" "$r" '"a":0'
check "split: second server" "$r" '"b":0'
r=$(q '{"query":"{ v: demo_v1_Greeter_GetVisitor(input:{id:\"x\"}) { __typename id visits count } }"}'); check "@key join" "$r" '"count":0'
check "@key join: other side" "$r" '"visits":0'
r=$(q '{"query":"query($i:JSON){ demo_v1_Greeter_GetVisitor(input:$i) { id visits count } }","variables":{"i":{"id":"y"}}}'); check "variables" "$r" '"id":"y"'
Q='query($x:Boolean!){ v: demo_v1_Greeter_GetVisitor(input:{id:\"x\"}) { id ...F @include(if:$x) } } fragment F on Visitor { visits count }'
r=$(q "{\"query\":\"$Q\",\"variables\":{\"x\":true}}"); check "named fragment, included" "$r" '"count":0'
r=$(q "{\"query\":\"$Q\",\"variables\":{\"x\":false}}"); check "named fragment, skipped" "$r" '"id":"x"' '"count"'
r=$(q '{"query":"{ v: demo_v1_Greeter_GetVisitor(input:{id:\"x\"}) { id ... on Other { secret } visits } }"}'); check "fragment on another type is dropped" "$r" '"visits"' 'secret'
r=$(q '{"query":"mutation { t: demo_v1_Greeter_Touch(input:{id:\"x\"}) { id visits count } }"}'); check "mutation + join (write runs once)" "$r" '"visits":1'
r=$(q '{"query":"mutation { t: demo_v1_Greeter_Touch(input:{id:\"x\"}) { visits } }"}'); check "second mutation increments" "$r" '"visits":2'
r=$(q '{"query":"mutation { demo_v1_Greeter_Visit demo_v1_Counter_Inc }"}'); check "mutation across servers is refused" "$r" 'only read-only queries'
r=$(q '{"query":"{ no_such_field }"}'); check "unknown field is an error" "$r" 'errors'
r=$(curl -s -XPOST localhost:8793/_reaxt/graphql -H 'content-type: text/plain' -d '{"query":"mutation { demo_v1_Greeter_Visit }"}'); check "subgraph action without x-reaxt is refused (CSRF)" "$r" 'missing x-reaxt header' '"demo_v1_Greeter_Visit":3'
echo; [ $fail -eq 0 ] && echo "ALL OK" || { echo "FAILED: $fail"; exit 1; }
