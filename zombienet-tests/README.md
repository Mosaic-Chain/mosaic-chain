# Zombienet

## (Optional for local testing) Get zombienet executable for your OS

```bash
curl -L -o zombienet-tests/scripts/zombienet https://github.com/paritytech/zombienet/releases/download/v1.3.98/zombienet-linux-x64
```

## (Optional for local testing) Spawn and Test network

```bash
zombienet -p native spawn <zombienet-test-file.toml>
```

```bash
zombienet -p native test <zombienet-test-file.zndsl>
```

## (Optional for local testing) Building chain-spec file

```bash
./target/release/mosaic-testnet-solo build-spec > chain-spec-plain.json
```

```bash
./target/release/mosaic-testnet-solo build-spec --chain chain-spec-plain.json --raw > mosaic-testnet-solo.json
```

## .toml file contents

.toml, .json and .yaml format is accepted here, currently it is running .toml.  
Complete list of commands: <https://paritytech.github.io/zombienet/network-definition-spec.html>

### [settings]

Provider is set to ***native***, because by default it will use ***kubernetes***. This setting could be deleted, but then in the spawning command ***-p native*** must be included.  

### [relaychain]

By default zombienet uses rococo-local testchain, but with changing the default_command and providing a chain spec file to it, it will run the test with mosaic-testnet-solo's release build.  
We run this as a "relaychain" currently because zombienet doesn't provide a "solochain" testing functionality. However zombienet is still useful for testing the functionalities which our chain has in common with a relaychain.

### [[relaychain.nodes]]

Specifies the name of the node we want to run in the relaychain, by default these are validators.

### [parachain]

In the future when the chain is converted to a parachain we could also use zombienet with a ``rococo-local`` relaychain.

## .zndsl file contents

.zndsl format is unique to zombienet. It uses natural language style, so it is simple and easily readable. This will run the .toml file chained to this and uses that to run the tests written in the file.
Complete list of commands: <https://paritytech.github.io/zombienet/cli/test-dsl-definition-spec.html>  

**Network:**  
Points to the .zndsl's associated .toml/.json/.yaml file.  

**Creds:**  
Used for kubernetes provider only. Won't run if deleted so it's just sitting there.  

**# well known functions**  
Tests if our 6 validators are up and running.

**# logs**  
Reads the temporary log files if they imported 50 blocks within the given time frame. The given time acts as a timeout, it will stop the test if it exceeds the time limit.
