#!/usr/bin/env python3
"""Generate a deterministic opaque 1024px Crimson Core app icon."""
import json, math, pathlib, struct, zlib

root = pathlib.Path(__file__).resolve().parent / "build" / "CrimsonAssets.xcassets"
icon = root / "AppIcon.appiconset"
icon.mkdir(parents=True, exist_ok=True)
size = 1024
def mix(a,b,t):
    t = max(0.0,min(1.0,t))
    return tuple(round(x+(y-x)*t) for x,y in zip(a,b))
def pixel(x,y):
    dx,dy=x-511.5,y-511.5
    d=math.hypot(dx,dy)
    base=mix((28,28,30),(20,20,22),d/730)
    red=(238,52,52)
    ring=abs(d-236)
    axis=min(abs(dx),abs(dy))
    reach=max(abs(dx),abs(dy))
    cross=max(axis-7,160-reach,reach-335)
    distance=min(ring-7,cross,d-10)
    base=mix(base,red,0.10*max(0,1-max(0,distance)/40))
    return mix(base,red,0.5-distance)
raw=bytearray()
for y in range(size):
    raw.append(0)
    for x in range(size):
        raw.extend(pixel(x,y))
def chunk(tag,data):
    return struct.pack(">I",len(data))+tag+data+struct.pack(">I",zlib.crc32(tag+data)&0xffffffff)
png=b"\x89PNG\r\n\x1a\n"+chunk(b"IHDR",struct.pack(">IIBBBBB",size,size,8,2,0,0,0))+chunk(b"IDAT",zlib.compress(raw,9))+chunk(b"IEND",b"")
(icon/"AppIcon.png").write_bytes(png)
(icon/"Contents.json").write_text(json.dumps({"images":[{"filename":"AppIcon.png","idiom":"universal","platform":"ios","size":"1024x1024"}],"info":{"author":"xcode","version":1}},indent=2))
(root/"Contents.json").write_text(json.dumps({"info":{"author":"xcode","version":1}}))
print("Generated",icon/"AppIcon.png")
