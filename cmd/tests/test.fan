client user {
    scheme: https,
    host: "httpbin.org",
    requests: {
        get: {
            path: "/get",
            method: GET,
            headers: [["a", "b"]],
            params: [
                ["key", "value"],
                ["variable", "hello \(variable)"],
                ["expr", "hello \(a + b)"],
            ],
            asserts: [status == 200],
        },
        post: {
            path: "/post",
            method: POST,
            params: [["a", "b"]],
            asserts: [status == 200],
        },
        postForm: {
            path: "/post",
            method: POST,
            headers: [
                ["a", "b"],
                ["Content-Type", "application/x-www-form-urlencoded"],
            ],
            params: [["key", "value"]],
            body: [["c", "d"]],
            asserts: [status == 200],
        },
        postMultipart: {
            path: "/post",
            method: POST,
            headers: [
                ["a", "b"],
                ["Content-Type", "multipart/form-data"],
            ],
            params: [["key", "value"]],
            body: [["c", "d"], ["f", "lib.rs"]],
            asserts: [status == 200],
        },
        postJson: {
            path: "/post",
            method: POST,
            headers: [
                ["a", "b"],
                ["Content-Type", "application/json"],
            ],
            params: [["key", "value"]],
            body: {
                name: "Gauss",
                age: 6,
                address: {
                    street: "19 Hear Sea Street",
                    city: "DaLian",
                },
                phones: ["+86 13098767890", "+86 15876567890"],
            },
            asserts: [status == 200],
        },
    },
}

client testApi {
    scheme: https,
    host: "httpbin.org",
    requests: {
        getIp: {
            path: "/ip",
            method: GET,
            headers: [["a", "b"]],
            params: [["key", "value"]],
            asserts: [status == 200],
        },
        postJsonUseLiteralStyle: {
            path: "/post",
            method: POST,
            headers: [
                ["a", "b"],
                ["Content-Type", "application/json"],
            ],
            params: [["key", "value"]],
            body: {
                name: "Gauss",
                age: 6,
                address: {
                    street: "19 Hear Sea Street",
                    city: "DaLian",
                },
                phones: ["+86 13098767890", "+86 15876567890"],
            },
            asserts: [status == 200],
        },
    },
}

test call {
    let variable = "world";
    let a = 1;
    let b = 2;
    let response = user.get();
    response.status
}
