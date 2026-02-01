import { spawn } from "node:child_process";
import { json } from "node:stream/consumers";

const bin = process.argv[1];
let buffer = Buffer.alloc(0);

const args = ["-l", ".logs/sqls.log", ...process.argv.slice(2)];

const sqls = spawn(bin, args, {
    stdio: ["pipe", "pipe", "inherit"],
});

// Zed (stdin) -> SQLS (stdin)
process.stdin.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    while (true) {
        const str = buffer.toString("utf-8");

        const match = str.match(/Content-Length:(\d+)\r\n\r\n/);

        if (!match) break;

        const contentLength = parseInt(match[1], 10);
        const headerLength = match[0].length;
        const totalLength = headerLength + contentLength;

        if (buffer.length < totalLength) break;

        const jsonStr = buffer
            .subarray(headerLength, totalLength)
            .toString("utf-8");
        const message = JSON.parse(message, null, 2) + "\n";

        if (message.method === "initialize") {
            process.stderr.write("\n[DEBUG] Initialize:");
            process.stderr.write(JSON.stringify(message, null, 2) + "\n");
        }
        sqls.stdin.write(buffer.subarray(0, totalLength));

        buffer = buffer.subarray(totalLength);
    }
});

// SQLS (stdout) -> Zed (stdout)
sqls.stdout.on("data", (chunk) => {
    process.stdout.write(chunk);
});

sqls.on("exit", (code) => process.exit(code));
process.on("SIGINT", () => sqls.kill());
