env local {
    scheme: http,
    host: "127.0.0.1",
    port: 8080
}

env staging {
    scheme: https,
    host: "api.example.com",
    port: 8443
}

api user {
    scheme: env.scheme,
    host: env.host,
    port: env.port,

    headers: {
        Accept: "application/json",
        Authorization: "Bearer \(secret("API_TOKEN"))"
    },

    get(id: int) {
        method: GET,
        path: "/users/\(id)",
        params: {
            include: "profile",
            tag: ["rust", "http"]
        }
    },

    query(page: int) {
        method: QUERY,
        path: "/users",
        json: {
            page: page,
            tag: ["rust", "http"]
        }
    },

    create(name: string, age: int) {
        method: POST,
        path: "/users",
        json: {
            name: name,
            age: age,
            "key world": "This is value."
        }
    },

    updateAvatar(id: int, avatar: string) {
        method: POST,
        path: "/users/\(id)/avatar",
        multipart: {
            source: "profile",
            avatar: file(avatar)
        }
    },

    submitForm(name: string, email: string) {
        method: POST,
        path: "/newsletter",
        form: {
            name: name,
            email: email
        }
    },

    webhook(payload: string) {
        method: POST,
        path: "/webhook",
        headers: {
            "Content-Type": "text/plain; charset=utf-8"
        },
        text: payload
    },

    postText() {
        method: POST,
        path: "/text",
        headers: {
            "Content-Type": "text/plain; charset=utf-8"
        },
        text: "hello"
    },

    xml(id: int, name: string) {
        method: POST,
        path: "/xml",
        headers: {
            "Content-Type": "application/xml; charset=utf-8"
        },
        text: `
        <?xml version="1.0" encoding="UTF-8"?>
        <order>
            <id>\(id)</id>
            <name>\(name)</name>
        </order>
        `
    },

    sendFile() {
        method: POST,
        path: "/file",
        headers: {
            "Content-Type": "application/xml"
        },
        file: file("./folder/file.xml")
    }
}

// Create a user and verify it can be fetched.
@smoke @users
test createUser {
    let created = user.create("Gauss", 6);
    expect created.status == 201;
    expect created.header("content-type").contains("application/json");
    expect created.json.id != null;

    let fetched = user.get(created.json.id);
    expect fetched.status == 200;
    expect fetched.json.id == created.json.id;
    expect fetched.json.name == "Gauss";
}

// Basjoofan DSL v1.0 semantics
//
// A .fan source file can define environments, API clients, and executable tests.
// Definitions may appear in the same file or be split across files loaded by the
// CLI. Names for environments, APIs, requests, parameters, and tests must be
// identifiers. Object entries, array items, request definitions, parameters, and
// call arguments are comma-separated; a trailing comma is optional. Statements
// inside a test end with semicolons.
//
// ENVIRONMENTS
//
// env <name> { ... } defines a named set of runtime configuration values. The CLI
// selects one environment with --env. Inside expressions, env.<field> reads a
// field from the selected environment. Environment values may be strings,
// numbers, booleans, or identifiers such as http and https.
//
// API CLIENTS
//
// api <name> { ... } defines a reusable HTTP client. scheme and host are required.
// scheme supports http and https. port is optional; when omitted, HTTP uses 80 and
// HTTPS uses 443. headers defines headers shared by every request in the API.
//
// API-level and request-level headers are merged by case-insensitive field name.
// A request-level header replaces an API-level header with the same name. Header
// names containing characters such as '-' must be quoted, for example
// "Content-Type". Header values are expressions. An array value emits one header
// line for each scalar array item.
//
// REQUESTS AND ARGUMENTS
//
// A request is declared inside an API as <name>(<parameters>) { ... } and called
// as <api>.<request>(<value>, ...). Every declared parameter must be supplied
// exactly once in declaration order. Supported parameter types are integer,
// float, boolean, string, array, and map; int and bool are accepted aliases.
// file is retained as a compatibility type for file(path) values. Arguments are evaluated and
// type-checked before the request is sent. Request names must be unique within an
// API.
//
// method and path are required request fields. Supported methods are GET, QUERY,
// POST, PUT, PATCH, DELETE, OPTIONS, HEAD, TRACE, and CONNECT. Interpolated path
// values are encoded as URL path-segment data. params defines URL query fields;
// names and values are URL encoded. An array value emits the same query name once
// for each scalar array item.
//
// REQUEST BODIES
//
// A request may define at most one body field: json, form, multipart, text, or
// file.
//
// json evaluates its value, serializes it as JSON, and adds
// Content-Type: application/json unless the request already supplies that header.
// JSON object keys may be identifiers or quoted strings. JSON arrays remain
// arrays and are not expanded into repeated fields.
//
// form encodes fields as application/x-www-form-urlencoded and adds the matching
// Content-Type header unless already supplied. An array value emits the same form
// field once for each scalar array item.
//
// multipart creates multipart/form-data with an automatically generated boundary.
// Scalar values create text parts, file(path) references create file parts, and
// arrays emit one part for each item.
//
// text sends a string or raw string without additional serialization. file sends
// the bytes referenced by file(path). text and file bodies require an explicit
// Content-Type header.
//
// STRINGS AND INTERPOLATION
//
// Double-quoted strings process escapes such as \n, \r, \t, \", and \\.
// Backtick-delimited raw strings preserve ordinary characters and may span
// multiple lines. Common indentation is removed from multiline raw strings.
// Both string forms support \(expression) interpolation. Inside a raw string,
// \` represents a literal backtick.
//
// NATIVE FUNCTIONS
//
// Native functions provide values or behavior supplied by the runtime. Function
// arguments are expressions and an invalid argument is an evaluation error.
// Basjoofan DSL v1.0 provides these native functions:
//
// secret(name) accepts one string containing a secret name and returns the secret
// as a string. The runtime reads the value from its secret environment. A missing
// secret is an evaluation error. Secret values may be used in expressions,
// headers, and bodies, but must be redacted from logs, reports, and diagnostics.
//
// file(path) accepts one string path and returns a reference to that file. It is
// used by file request bodies and multipart file parts. The file is read when the
// request is prepared; an unreadable or missing file is an evaluation error.
// A file reference is not a separate DSL value type.
//
// RESPONSES
//
// A successful request returns a response with status, headers, body, duration,
// request metadata, and json when the response contains valid JSON. Header lookup
// through response.header(name) is case-insensitive and supports repeated header
// values. response.header(name).contains(value) succeeds when any matching value
// contains the requested text.
//
// response.json uses a standard JSON parser. Accessing json on a non-JSON or
// invalid JSON response is an error. A missing field, array index, map key, or
// header returns null, so optional values can be checked with != null. Accessing
// a field or index on an incompatible value remains an error.
//
// TESTS AND EXPECTATIONS
//
// test <name> { ... } defines an executable test. let binds a value for later
// statements in that test. expect requires a boolean expression. false, an
// evaluation error, a transport error, or a response decoding error fails the
// current test. The CLI exits with a non-zero status when any selected test fails.
//
// One or more @tag annotations may appear immediately before a test. Tags classify
// tests and allow the CLI to select them with an @tag positional selector. // starts a line comment;
// comments do not affect execution.
