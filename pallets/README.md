# Pallets

Whatevers.

## Diagrams

You can use PlantUML to view diagrams. In Visual Studio code you can install the extension typing

```plain
ext install jebbs.plantuml
```

into the `Ctrl+Shift+P` popup, or using the extensions manager. In order to generate previews of these
diagrams using `Ctrl+D` you need to run a PlantUML server on localhost tough. You can install that the
easiest using docker:

```sh
docker run plantuml/plantuml-server:jetty
```

This will bind to <http://localhost:8080> to run the PlantUML server, so you do not need to touch the
configuration of the extension.

If you are too lazy to do these, the sources of the diagrams are quite readable, or you can paste them
into a new <https://hackmd.io/> note, surrounding it with

````plain
```plantuml
@startuml
...
@enduml
```
````
