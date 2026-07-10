env local {
    scheme: http,
    host: "127.0.0.1",
    port: 8080
}

env test {
    scheme: https,
    host: "api.example.com"
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

    updateAvatar(id: int, avatar: file) {
        method: POST,
        path: "/users/\(id)/avatar",
        multipart: {
            source: "profile",
            avatar: avatar
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
            Content-Type: "text/plain; charset=utf-8"
        },
        raw: payload
    },

    postText() {
        method: POST,
        path: "/text",
        headers: {
            Content-Type: "text/plain; charset=utf-8"
        },
        raw: "hello"
    },

    xml(id: int, name: string) {
        method: POST,
        path: "/xml",
        headers: {
            Content-Type: "application/xml; charset=utf-8"
        },
        raw: `
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
            Content-Type: "application/xml"
        },
        raw: file("./folder/file.xml")
    }
}

// Create a user and verify it can be fetched.
@smoke @users
test createUser {
    let created = user.create(name: "Gauss", age: 6);
    expect created.status == 201;
    expect created.header("content-type").contains("application/json");
    expect created.json.id exists;

    let fetched = user.get(id: created.json.id);
    expect fetched.status == 200;
    expect fetched.json.id == created.json.id;
    expect fetched.json.name == "Gauss";
}

// Basjoofan DSL v1.0 semantics
//
// Source files define environments, API clients, and executable tests.
//
// Object fields and API request definitions are separated by commas.
// The last item has no trailing comma. Arrays and call arguments also use commas.
// Test statements are separated by semicolons.
//
// env defines named runtime configuration. The CLI selects one environment.
// scheme is required and currently supports http and https.
// host is required. port is optional; omitted ports resolve to 80 for http
// and 443 for https. env.<name> reads a field from the selected environment.
//
// api defines a reusable HTTP client. scheme, host, port, and headers are
// client defaults. Request headers are merged with client headers by
// case-insensitive name; request headers override client headers.
//
// An API request is declared as name(parameters) { ... } and called as
// api.name(argument: value). Parameters are type-checked before execution.
// Request names must be unique within an API.
//
// method and path are required request fields. path interpolations are URL
// path-segment encoded. params are URL query parameters and are URL encoded.
// A scalar array in params expands into repeated query keys.
//
// headers use static HTTP field names, including names with '-'. Header values
// are expressions. A scalar array in headers expands into repeated field lines.
// secret(name) resolves a runtime secret and is redacted from logs and reports.
//
// json serializes its object value using a standard JSON encoder and adds
// Content-Type: application/json. JSON keys may be identifiers or static
// quoted strings. JSON arrays remain JSON arrays.
//
// form serializes scalar fields as application/x-www-form-urlencoded.
// A scalar array in form expands into repeated form fields.
//
// multipart creates multipart/form-data with an automatically generated boundary.
// String and scalar values create text parts; file values create file parts.
// Arrays create repeated multipart parts.
//
// raw sends a string, template string, bytes, or file value without serialization.
// raw requests must explicitly provide Content-Type in headers.
// file(path) loads a file value from the supplied path.
//
// Double-quoted strings and backtick template strings support \(expression)
// interpolation. Backtick strings may span multiple lines and remove common
// indentation. A literal backtick is written as \`.
//
// QUERY is a safe and idempotent HTTP method with a request representation.
// QUERY requests require Content-Type; json, form, multipart, or explicit raw
// headers provide it.
//
// A request returns a response with status, headers, body, json, duration,
// and request metadata. response.header(name) performs case-insensitive lookup.
// header(...).contains(value) checks a single or repeated header value.
//
// response.json is parsed with a standard JSON parser. Accessing it for a
// non-JSON or invalid JSON response is an error. exists safely tests whether
// a response selector is present; ordinary missing-field access is an error.
//
// expect evaluates one boolean expression. A false result, evaluation error,
// transport error, or response decoding error fails the current test.
// The CLI returns a non-zero exit status when any test fails.
//
// @tag annotations classify the following test and support CLI filtering.
// // comments are non-semantic documentation.
