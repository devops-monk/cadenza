import sys, math
from PIL import Image, ImageDraw, ImageFilter, ImageChops
S = 4096  # supersampled, downscaled to 1024
out = sys.argv[1]

def lerp(a, b, t): return tuple(int(a[i] + (b[i]-a[i])*t) for i in range(3))
def gradient(size, stops, angle=135):
    w, h = size
    img = Image.new("RGB", size)
    px = img.load()
    # diagonal gradient computed on a small image then resized for speed
    small = Image.new("RGB", (256, 256)); sp = small.load()
    ca, sa = math.cos(math.radians(angle)), math.sin(math.radians(angle))
    for y in range(256):
        for x in range(256):
            t = ((x/255-0.5)*ca + (y/255-0.5)*sa) / 1.0 + 0.5
            t = max(0, min(1, t))
            for i in range(len(stops)-1):
                p0, c0 = stops[i]; p1, c1 = stops[i+1]
                if p0 <= t <= p1:
                    sp[x, y] = lerp(c0, c1, (t-p0)/(p1-p0) if p1 > p0 else 0); break
    return small.resize(size, Image.BICUBIC)

# macOS icon grid: 824px body inside 1024 canvas -> scale by 4
pad = int(100*4); body = S - 2*pad; r = int(185*4)
mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle([pad, pad, S-pad, S-pad], radius=r, fill=255)

bg = gradient((S, S), [(0, (24, 16, 48)), (1, (8, 6, 18))], angle=90)
canvas = Image.new("RGBA", (S, S), (0, 0, 0, 0))
canvas.paste(bg, (0, 0), mask)

accent = gradient((S, S), [(0, (255, 176, 72)), (0.45, (255, 72, 140)), (1, (124, 92, 255))], angle=120)

cx = cy = S // 2
# soft glow
glow = Image.new("L", (S, S), 0)
ImageDraw.Draw(glow).ellipse([cx-1300, cy-1300, cx+1300, cy+1300], fill=120)
glow = glow.filter(ImageFilter.GaussianBlur(380))
glow = ImageChops.multiply(glow, mask)
canvas = Image.composite(Image.merge("RGBA", (*accent.split(), glow)), canvas, glow.point(lambda v: 255 if v else 0)) if False else canvas
canvas.alpha_composite(Image.merge("RGBA", (*accent.split(), glow)))

# ring: an open "C" arc (Cadenza) with gap on the right
shape = Image.new("L", (S, S), 0)
d = ImageDraw.Draw(shape)
R, T = 1150, 210
d.arc([cx-R, cy-R, cx+R, cy+R], start=40, end=320, fill=255, width=T)
# rounded arc caps
for ang in (40, 320):
    a = math.radians(ang); rr = R - T/2
    x, y = cx + rr*math.cos(a), cy + rr*math.sin(a)
    d.ellipse([x-T/2, y-T/2, x+T/2, y+T/2], fill=255)

# waveform bars inside, crescendo then release
heights = [0.22, 0.42, 0.7, 1.0, 0.78, 0.5, 0.3]
bw, gap = 150, 92
total = len(heights)*bw + (len(heights)-1)*gap
x0 = cx - total/2 - 95
H = 1180
for i, h in enumerate(heights):
    x = x0 + i*(bw+gap); hh = H*h
    d.rounded_rectangle([x, cy-hh/2, x+bw, cy+hh/2], radius=bw/2, fill=255)

# sparkle at the arc's opening (the "AI" spark)
sx, sy, sr = cx + 1030, cy - 20, 290
pts = []
for k in range(8):
    a = math.radians(k*45 - 90); rad = sr if k % 2 == 0 else sr*0.28
    pts.append((sx + rad*math.cos(a), sy + rad*math.sin(a)))
d.polygon(pts, fill=255)

shape_img = Image.merge("RGBA", (*accent.split(), shape))
# subtle shadow
shadow = shape.filter(ImageFilter.GaussianBlur(60)).point(lambda v: int(v*0.55))
canvas.alpha_composite(Image.merge("RGBA", (Image.new("L",(S,S),0),)*3 + (shadow,)), (0, 40))
canvas.alpha_composite(shape_img)
# top highlight sheen
sheen = Image.new("L", (S, S), 0)
ImageDraw.Draw(sheen).rounded_rectangle([pad, pad, S-pad, pad+body*0.5], radius=r, fill=22)
sheen = ImageChops.multiply(sheen.filter(ImageFilter.GaussianBlur(220)), mask)
canvas.alpha_composite(Image.merge("RGBA", (Image.new("L",(S,S),255),)*3 + (sheen,)))
# re-clip to body
alpha = ImageChops.multiply(canvas.split()[3], mask)
canvas.putalpha(alpha)
canvas.resize((1024, 1024), Image.LANCZOS).save(out)
print("saved", out)
