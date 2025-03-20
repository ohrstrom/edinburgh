#! /usr/env/bin python3

# /// script
# dependencies = [
#   "websockets",
# ]
# ///

import asyncio
import websockets

TCP_HOST = "edi-ch.digris.net"
TCP_PORT = 8855
WS_PORT = 8855

async def handle_client(websocket):
    reader, writer = await asyncio.open_connection(TCP_HOST, TCP_PORT)

    async def forward_tcp_to_ws():
        while True:
            data = await reader.read(4096)
            print(f"fwd: {len(data)} bytes")
            if not data:
                break
            await websocket.send(data)

    async def forward_ws_to_tcp():
        async for message in websocket:
            if isinstance(message, bytes):
                print(f"rcv: {len(message)} bytes")
                writer.write(message)
                await writer.drain()

    await asyncio.gather(forward_tcp_to_ws(), forward_ws_to_tcp())

async def main():
    async with websockets.serve(handle_client, "0.0.0.0", WS_PORT):
        await asyncio.Future()

asyncio.run(main())
