ganache-cli \
  -l 8000000 \
  -e 100000000 \
  -m "minimum brain detail minimum leader slight correct length document focus grain vault evoke credit apart" \
> /dev/null &
ganache_pid=$!
# run ethereum tests
kill -9 $ganache_pid
