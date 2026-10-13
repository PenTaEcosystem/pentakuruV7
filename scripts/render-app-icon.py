"""Render the code-owned PentaKuru pentagon identity for native app windows."""
from pathlib import Path
from math import sin, cos, pi
from PIL import Image, ImageDraw

root = Path(__file__).resolve().parent.parent
size = 1024
im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
points = [(size/2 + size*.43*cos(-pi/2+i*2*pi/5), size/2 + size*.43*sin(-pi/2+i*2*pi/5)) for i in range(5)]
draw = ImageDraw.Draw(im)
draw.polygon(points, fill=(0, 0, 0, 255))
draw.line(points+[points[0]], fill=(255, 255, 255, 255), width=round(size*.045), joint="curve")
im.resize((256,256),Image.Resampling.LANCZOS).save(root/"assets/pentakuru-icon.png")
im.save(root/"assets/pentakuru-icon.ico",sizes=[(n,n) for n in [16,24,32,48,64,128,256]])
(root/"assets/pentakuru-icon.rgba").write_bytes(im.resize((64,64),Image.Resampling.LANCZOS).tobytes())
svg_points=" ".join(f"{x/size*100:.3f},{y/size*100:.3f}" for x,y in points)
(root/"assets/pentakuru-icon.svg").write_text(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><polygon points="{svg_points}" fill="black" stroke="white" stroke-width="4.5" stroke-linejoin="round"/></svg>\n',encoding="utf-8")
icon = im.resize((256,256),Image.Resampling.LANCZOS)
for name, dimensions, logo_size, position in [
    ("pentakuru-wizard-small.bmp",(64,64),(54,54),(5,5)),
    ("pentakuru-wizard.bmp",(164,314),(124,124),(20,74)),
]:
    canvas=Image.new("RGB",dimensions,(23,23,25))
    logo=icon.resize(logo_size,Image.Resampling.LANCZOS)
    canvas.paste(logo,position,logo)
    canvas.save(root/"assets"/name)
