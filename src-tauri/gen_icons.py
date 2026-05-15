import struct, zlib, os

BASE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "icons")

def create_png(width, height):
    """Create a PNG with blue gradient circle"""
    def chunk(chunk_type, data):
        c = chunk_type + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)

    ihdr = struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)

    raw = b''
    for y in range(height):
        raw += b'\x00'
        for x in range(width):
            cx, cy = width // 2, height // 2
            dx, dy = (x - cx) / cx, (y - cy) / cy
            dist = (dx*dx + dy*dy) ** 0.5
            if dist > 1.0:
                raw += struct.pack('BBBB', 0, 0, 0, 0)
            else:
                r = int(30 + 40 * (1 - dist))
                g = int(100 + 100 * (1 - dist))
                b = int(200 + 55 * (1 - dist))
                raw += struct.pack('BBBB', r, g, b, 255)

    compressed = zlib.compress(raw)
    png = b'\x89PNG\r\n\x1a\n'
    png += chunk(b'IHDR', ihdr)
    png += chunk(b'IDAT', compressed)
    png += chunk(b'IEND', b'')
    return png


def create_bmp_ico(sizes_and_data):
    """
    Create a classic BMP-based ICO file.
    sizes_and_data: list of (width, height, bmp_data) tuples
    """
    num_images = len(sizes_and_data)

    # ICO header
    ico = struct.pack('<HHH', 0, 1, num_images)

    # Calculate offsets
    header_size = 6 + 16 * num_images
    entries = b''
    image_data = b''
    offset = header_size

    for width, height, bmp in sizes_and_data:
        # ICO directory entry
        w = 0 if width >= 256 else width
        h = 0 if height >= 256 else height
        entries += struct.pack('<BBBBHHIH', w, h, 0, 0, 1, 32, len(bmp), offset)
        image_data += bmp
        offset += len(bmp)

    return ico + entries + image_data


def create_bmp_ico_image(size):
    """Create BMP data for one ICO image entry (BITMAPINFOHEADER + pixels + AND mask)"""
    w = size
    h = size
    bpp = 32

    # BITMAPINFOHEADER (40 bytes)
    # Height is doubled for ICO: first half = XOR (image), second half = AND (mask)
    header = struct.pack('<IiiHHIIiiII',
        40,              # biSize
        w,               # biWidth (signed)
        h * 2,           # biHeight (signed, doubled for ICO)
        1,               # biPlanes
        bpp,             # biBitCount
        0,               # biCompression (BI_RGB)
        0,               # biSizeImage (can be 0 for BI_RGB)
        0,               # biXPelsPerMeter
        0,               # biYPelsPerMeter
        0,               # biClrUsed
        0,               # biClrImportant
    )

    # XOR image data (BGRA, bottom-up)
    xor_data = b''
    for y in range(h - 1, -1, -1):  # bottom-up
        for x in range(w):
            cx, cy = w // 2, h // 2
            dx, dy = (x - cx) / cx, (y - cy) / cy
            dist = (dx*dx + dy*dy) ** 0.5
            if dist > 1.0:
                # Transparent - XOR = black, AND = 0 (transparent)
                xor_data += struct.pack('BBBB', 0, 0, 0, 0)
            else:
                r = int(30 + 40 * (1 - dist))
                g = int(100 + 100 * (1 - dist))
                b_val = int(200 + 55 * (1 - dist))
                xor_data += struct.pack('BBBB', b_val, g, r, 0)  # BGRA

    # AND mask (1 bit per pixel, bottom-up, rows padded to 4 bytes)
    # 0 = opaque, 1 = transparent
    and_row_bytes = ((w + 31) // 32) * 4
    and_data = b''
    for y in range(h - 1, -1, -1):  # bottom-up
        row = bytearray(and_row_bytes)
        for x in range(w):
            cx, cy = w // 2, h // 2
            dx, dy = (x - cx) / cx, (y - cy) / cy
            dist = (dx*dx + dy*dy) ** 0.5
            if dist > 1.0:
                # Set bit to 1 (transparent)
                byte_idx = x // 8
                bit_idx = 7 - (x % 8)
                row[byte_idx] |= (1 << bit_idx)
        and_data += bytes(row)

    return header + xor_data + and_data


# Generate BMP-based ICO with multiple sizes
ico_sizes = [16, 32, 48]
bmp_entries = []
for s in ico_sizes:
    bmp = create_bmp_ico_image(s)
    bmp_entries.append((s, s, bmp))
    print(f"  BMP {s}x{s}: {len(bmp)} bytes")

ico_data = create_bmp_ico(bmp_entries)
ico_path = os.path.join(BASE, "icon.ico")
with open(ico_path, 'wb') as f:
    f.write(ico_data)
print(f"\nicon.ico: {len(ico_data)} bytes total")

# Generate PNG files for other icon slots
for size, name in [(32, "32x32.png"), (128, "128x128.png"), (256, "128x128@2x.png")]:
    png = create_png(size, size)
    with open(os.path.join(BASE, name), 'wb') as f:
        f.write(png)
    print(f"{name}: {len(png)} bytes")

# icon.icns placeholder
import shutil
shutil.copy(os.path.join(BASE, "128x128.png"), os.path.join(BASE, "icon.icns"))
print("icon.icns: copied from 128x128.png")
