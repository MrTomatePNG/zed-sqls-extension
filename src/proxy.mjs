import { spawn } from "node:child_process";
import { log } from "node:console";
import { appendFileSync } from "node:fs";
const bin = process.argv[1];
const args = ["-l", ".logs/sqls.log", ...process.argv.slice(2)];

const sqls = spawn(bin, args, {
    stdio: ["pipe", "pipe", "inherit"],
});

let buffer = Buffer.alloc(0);

function formatLSP(obj) {
    const json = JSON.stringify(obj);
    return `Content-Length: ${Buffer.byteLength(json, "utf8")}\r\n\r\n${json}`;
}

process.stdin.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);

    while (true) {
        const str = buffer.toString("utf-8");
        // Adicionado um espaço opcional (\s*) após os dois pontos
        const match = str.match(/Content-Length:\s*(\d+)\r\n\r\n/);

        if (!match) break;

        const contentLength = parseInt(match[1], 10);
        const headerLength = match[0].length;
        const totalLength = headerLength + contentLength;

        if (buffer.length < totalLength) break;

        const jsonStr = buffer
            .subarray(headerLength, totalLength)
            .toString("utf-8");

        try {
            // CORRIGIDO: Parse simples
            const message = JSON.parse(jsonStr);

            if (message.method === "initialize") {
                const logEntry = {
                    time: new Date().toISOString(),
                    message: message,
                };
                appendFileSync(
                    "initialize_debug.json",
                    JSON.stringify(logEntry, null, 2) + "\n---\n",
                );
            }
        } catch (e) {
            process.stderr.write("\n[ERROR] Falha ao parsear JSON\n");
        }

        // Envia os bytes originais para não corromper a mensagem
        sqls.stdin.write(buffer.subarray(0, totalLength));
        buffer = buffer.subarray(totalLength);
    }
});

sqls.stdout.on("data", (chunk) => {
    const message = JSON.parse(jsonStr);
    process.stdout.write(chunk);
});

sqls.on("exit", (code) => process.exit(code || 0));
process.on("SIGINT", () => sqls.kill());
