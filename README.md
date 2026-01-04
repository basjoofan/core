# Basjoofan
Cloud Native API Testing and Performance Testing Service

Basjoofan is a cloud-native interface and performance testing service. It enables rapid authoring and execution of interface test cases through a concise and intuitive scripting language, while supporting the complete testing workflow from local development to cloud deployment.

## ✨ Features
* Simple test script writing
* Intuitive HTTP request support without complex configuration
* Assertion syntax
* Containerized deployment for easy scaling

## 🚀 Start
Let's start with a simple GET request:
```
let host = "httpbin.org";

rq get`
GET https://{host}/get
`[status == 200]
```
Using the rq keyword to define a request named get and assert response status code equals 200.
```
test get {
let response = get->;
response.status
}
```
Using the test keyword to define a test block that assembles interface logic for executing test cases.

You can use the CLI tool with basjoofan test get to execute this test case. You can also add load testing parameters for performance testing, e.g., -t 100 -d 1m for 100 concurrent users running for 1 minute.
```
basjoofan test get -t 100 -d 1m
```

For VSCode users: test script files need to end with .fan extension. Once automatically recognized, executable test blocks will have a run button added. Click the run button to execute the test case.

Interested folks are welcome to try it out! I'd really appreciate any feedback you might have. Thanks!

## 📄 License
MIT and Apache 2.0, allowing free use, modification, and distribution.

## 🚧 Plans
* [flow](https://github.com/basjoofan/flow) Web service for manually or automatically executing test scripts in cloud resources (under development)
* [vsc](https://github.com/basjoofan/vsc) VSCode extension for developing test scripts (preview version available)
* [zed](https://github.com/basjoofan/zed) Zed extension for developing test scripts (not started)
