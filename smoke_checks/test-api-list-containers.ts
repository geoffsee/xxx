#!/usr/bin/env bun

const containersRequest = await fetch("http://localhost:3000/api/containers/list");

console.log(await containersRequest.json().catch(console.error));

