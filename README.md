# Basjoofan
API test and performance test tool

Basjoofan is a  interface and performance test tool. It enables rapid authoring and execution of interface test cases through a concise and intuitive scripting language, while supporting the complete testing workflow from local development to cloud deployment.

## ✨ Features
* Simple test script writing
* Intuitive HTTP request support without complex configuration
* Assertion syntax
* Containerized deployment for easy scaling

## 🚀 Start
Let's start with a simple GET request:
```fan
env local { scheme: https, host: "httpbin.org" }

api user {
    scheme: env.scheme,
    host: env.host,
    get() { method: GET, path: "/get" }
}

test get {
    let response = user.get();
    expect response.status == 200;
}
```
The CLI recursively loads `.fan` files. Select the target environment explicitly:
```
basjoofan test get --env local
```

Use an `@`-prefixed selector to run every test carrying a tag:
```
basjoofan test @smoke --env local
```

For VSCode users: test script files need to end with .fan extension. Once automatically recognized, executable test blocks will have a run button added. Click the run button to execute the test case.

Interested folks are welcome to try it out! I'd really appreciate any feedback you might have. Thanks!

## 📄 License
MIT and Apache 2.0, allowing free use, modification, and distribution.

## 🚧 Plans
* [flow](https://github.com/basjoofan/flow) Web service for manually or automatically executing test scripts in cloud resources (under development)
* [vsc](https://github.com/basjoofan/vsc) VSCode extension for developing test scripts (preview version available)
* [zed](https://github.com/basjoofan/zed) Zed extension for developing test scripts (not started)
