ids=(alice bob charlie dave eve ferdie)

max=6
for (( i=0; i < $max; i++ ))
do
	./target/release/node-template purge-chain --base-path /tmp/${ids[i]} --chain local -y

	./target/release/node-template \
	--base-path /tmp/${ids[i]} \
	--chain local \
	--${ids[i]} \
	--port $((30333 + i)) \
	--ws-port $((9945 +i)) \
	--unsafe-ws-external \
	--rpc-port $((9933 + i)) \
	--node-key 000000000000000000000000000000000000000000000000000000000000000$((1 + i)) \
	--validator \
	--rpc-methods=Unsafe \
	--rpc-cors=all \
	&
done

# sigkill on exit becasue tokio can go berserk occasionally
trap "trap - SIGTERM && kill -9 -- -$$" SIGINT SIGTERM EXIT

while true; do read; done