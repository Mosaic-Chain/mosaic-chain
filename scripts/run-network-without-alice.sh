ids=(bob charlie dave eve ferdie)

max=5
for (( i=0; i < $max; i++ ))
do
	./target/release/node-template purge-chain \
	--base-path /tmp/${ids[i]} \
	--chain local -y \

	./target/release/node-template \
	--base-path /tmp/${ids[i]} \
	--chain local \
	--${ids[i]} \
	--port $((30334 + i)) \
	--rpc-port $((9946 +i)) \
	--node-key 000000000000000000000000000000000000000000000000000000000000000$((2 + i)) \
	--validator \
	&
done

# sigkill on exit becasue tokio can go berserk occasionally
trap "trap - SIGTERM && kill -9 -- -$$" SIGINT SIGTERM EXIT

while true; do read; done