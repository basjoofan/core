# Basjoofan
A cloud native HTTP API test and performance test service

[core](https://github.com/basjoofan/core) Command line tool for executing test scripts (preview version available)

[vscode](https://github.com/basjoofan/vscode) SCode extension for developing test scripts (preview version available)

[flow](https://github.com/basjoofan/flow) Web service for manually or automatically executing test scripts in cloud resources (under development)

Let's start with a simple GET request:
```
let host = "httpbingo.org";

rq get`
GET https://{host}/get
Host: {host}
Connection: close
`[status == 200]
```
Using the rq keyword to define a request named get and assert that the response status code equals 200.
```
test get {
let response = get->;
response.status
}
```
Using the test keyword to define a test block that assembles interface logic for executing test cases.

You can use the CLI tool with basjoofan test get to execute this test case. You can also add load testing parameters for performance testing, e.g., -t 100 -d 1m for 100 concurrent users running for 1 minute.

For VSCode users: test script files need to end with .fan extension. Once automatically recognized, executable test blocks will have a run button added. Click the run button to execute the test case.

Interested folks are welcome to try it out! I'd really appreciate any feedback you might have. Thanks!
