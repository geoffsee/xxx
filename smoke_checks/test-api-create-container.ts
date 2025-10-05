#!/usr/bin/env bun

const API_BASE = "http://localhost:3000/api/containers";

async function main() {
    try {
        // Create a new container
        const createResponse = await fetch(`${API_BASE}/create`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                image: "alpine",
                command: ["echo", "hello from bun + podman"],
            }),
        });

        if (!createResponse.ok) {
            console.error(`❌ Failed to create container: ${createResponse.statusText}`);
            const text = await createResponse.text();
            console.error(text);
            return;
        }

        const createData = await createResponse.json();
        console.log("✅ Container created:");
        console.log(JSON.stringify(createData, null, 2));

        // List containers afterward
        const listResponse = await fetch(`${API_BASE}/list`);
        if (!listResponse.ok) {
            console.error(`❌ Failed to list containers: ${listResponse.statusText}`);
            return;
        }

        const containers = await listResponse.json();
        console.log("\n📦 Current Containers:");
        console.log(JSON.stringify(containers, null, 2));

    } catch (err) {
        console.error("Unhandled error:", err);
    }
}

await main();