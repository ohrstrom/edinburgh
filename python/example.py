#! /usr/bin/env python3

import asyncio
import websockets
from edinburgh import EDI

edi = EDI()

async def ws_reader():
    uri = "ws://127.0.0.1:9000/ws/edi-ch.digris.net/8855"
    async with websockets.connect(uri) as ws:
        while True:
            data = await ws.recv()
            if isinstance(data, bytes):
                edi.feed(data)

asyncio.run(ws_reader())