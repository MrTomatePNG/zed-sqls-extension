import { spawn } from "node:child_process";

const bin = process.argv[1];
const args = ["-t", ...process.argv.slice(2)];
const sqls = spawn(bin, args);

let buffer = Buffer.alloc(0);

function formatLSP(obj) {
    const json = JSON.stringify(obj);
    return `Content-Length: ${Buffer.byteLength(json, "utf8")}\r\n\r\n${json}`;
}

function parseLSP(chunk) {
    buffer = Buffer.concat([buffer, chunk]);
    const messages = [];

    while (true) {
        const str = buffer.toString("utf8");
        const match = str.match(/Content-Length: (\d+)\r\n\r\n/);

        if (!match) break;

        const contentLength = parseInt(match[1], 10);
        const headerLength = match[0].length;
        const totalLength = headerLength + contentLength;

        if (buffer.length < totalLength) break;

        const messageJson = buffer
            .subarray(headerLength, totalLength)
            .toString("utf8");
        try {
            messages.push(JSON.parse(messageJson));
        } catch (e) {
            console.error("Parse error:", e);
        }

        buffer = buffer.subarray(totalLength);
    }
    return messages;
}

function handleMessageFromServer(message) {
    if (message.id === 1 && message.result?.capabilities) {
        console.error(
            "[INIT RESPONSE] Capabilities:",
            JSON.stringify(message.result.capabilities, null, 2),
        );
    }

    return message;
}

function handleMessageFromZed(message) {
    if (message.method) {
        console.error(`[ZED->SQLS] ${message.method} (id: ${message.id})`);
    }

    // Intercept initialize to log it
    if (message.method === "initialize") {
        console.error(
            "[INIT REQUEST]",
            JSON.stringify(message.params, null, 2),
        );
        return message; // Pass through
    }

    if (message.method === "$/cancelRequest") {
        console.error("[IGNORED] $/cancelRequest");
        return null;
    }

    if (message.method === "workspace/didChangeConfiguration") {
        console.error("[CONFIG]", JSON.stringify(message.params, null, 2));
    }

    return message;
}

// Zed -> SQLS
process.stdin.on("data", (chunk) => {
    const messages = parseLSP(chunk);
    for (let msg of messages) {
        console.error("[RAW ZED]", msg.method || `Response ${msg.id}`);
        const filtered = handleMessageFromZed(msg);
        if (filtered) {
            sqls.stdin.write(formatLSP(filtered));
        }
    }
});

// SQLS -> Zed
sqls.stdout.on("data", (chunk) => {
    const messages = parseLSP(chunk);
    for (let msg of messages) {
        console.error("[RAW SQLS]", msg.method || `Response ${msg.id}`);

        if (msg.id === 1) {
            console.error(
                "[INIT RESPONSE] Capabilities:",
                JSON.stringify(msg.result?.capabilities, null, 2),
            );
        }

        const processed = handleMessageFromServer(msg);
        if (processed) {
            process.stdout.write(formatLSP(processed));
        }
    }
});

sqls.stderr.on("data", (data) => {
    process.stderr.write(data);
});

sqls.on("exit", () => process.exit());
