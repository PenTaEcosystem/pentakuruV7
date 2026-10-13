"""Reproducible native watch icons; no external images or fonts downloaded."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'vendor/penta-bip/assets/432x514-amazfit-bip-max'
S = 4
FONT = ROOT / 'assets/fonts/NotoSans-Regular.ttf'

def asset(name, size, kind, label='', active=False):
    im = Image.new('RGBA', (size[0]*S, size[1]*S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    c = '#ffffff' if active or kind in ('mic','stop','keyboard') else '#aaaaaa'
    def box(coords, fill=None, outline=c, radius=0):
        coords = tuple(int(v*S) for v in coords)
        if radius: d.rounded_rectangle(coords, int(radius*S), fill=fill, outline=outline, width=2*S)
        else: d.rectangle(coords, fill=fill, outline=outline, width=2*S)
    def line(points): d.line([(int(x*S),int(y*S)) for x,y in points], fill=c, width=2*S, joint='curve')
    def ellipse(coords): d.ellipse(tuple(int(v*S) for v in coords), outline=c, width=2*S)
    if kind == 'keyboard':
        box((0,0,size[0]-1,size[1]-1), '#202328', '#363a42', 14)
        box((16,15,40,32), radius=3)
        for x in (22,28,34): line([(x,20),(x,23)])
        line([(22,28),(34,28)])
        d.text((54*S,12*S),label,font=ImageFont.truetype(str(FONT),18*S),fill=c)
    elif kind in ('mic','stop'):
        box((0,0,size[0]-1,size[1]-1), '#353b47' if kind=='mic' else '#7f303b', None, 14)
        if kind=='stop': box((28,16,44,32), c, None, 3)
        else:
            box((31,9,41,27), radius=5)
            d.arc((26*S,16*S,46*S,33*S),0,180,fill=c,width=2*S)
            line([(36,33),(36,39)]);line([(31,39),(41,39)])
    elif kind=='refresh':
        d.arc((10*S,10*S,30*S,30*S),40,320,fill=c,width=2*S)
        line([(26,8),(30,12),(25,15)])
    else:
        box((0,0,size[0]-1,size[1]-1), '#2a3039' if active else '#15181d', None, 12)
        if kind=='note':
            box((31,6,49,27), radius=2)
            for y in (12,17,22): line([(35,y),(45,y)])
        elif kind=='search': ellipse((29,6,45,22));line([(43,20),(51,28)])
        elif kind=='ai':
            line([(40,5),(43,13),(51,16),(43,19),(40,27),(37,19),(29,16),(37,13),(40,5)])
        elif kind=='windows':
            box((28,7,52,23), radius=2);line([(40,24),(40,28)]);line([(34,28),(46,28)])
        f=ImageFont.truetype(str(FONT),13*S)
        d.text((40*S,35*S),label,font=f,fill=c,anchor='mt')
    im.resize(size,Image.Resampling.LANCZOS).save(OUT/name)

OUT.mkdir(parents=True, exist_ok=True)
for i,(kind,label) in enumerate([('note','Ghi chú'),('search','Tìm kiếm'),('ai','AI'),('windows','Windows')]):
    for active in (False,True): asset(f'chat_{i}_{"act" if active else "norm"}.png',(80,54),kind,label,active)
asset('chat_input.png',(300,48),'keyboard','Nhập câu hỏi / lệnh')
asset('chat_mic.png',(72,48),'mic')
asset('chat_stop.png',(72,48),'stop')
asset('chat_refresh.png',(40,40),'refresh')
print('Created 12 chat assets')

def input_icon(name, size, kind, background):
    # Opaque RGB backgrounds avoid firmware alpha/colour-key differences.
    image = Image.new('RGB', (size[0]*S,size[1]*S), '#0b0d10')
    draw = ImageDraw.Draw(image); cx=size[0]/2
    draw.rounded_rectangle((0,0,size[0]*S-1,size[1]*S-1),radius=12*S,fill=background)
    def box(coords, radius=0, fill=None):
        draw.rounded_rectangle(tuple(int(v*S) for v in coords),radius=int(radius*S),outline='#ffffff',fill=fill,width=2*S)
    def line(points): draw.line([(int(x*S),int(y*S)) for x,y in points],fill='#ffffff',width=2*S)
    if kind=='keyboard':
        box((cx-20,15,cx+20,41),4)
        for y in (22,28):
            for x in (-12,-4,4,12): line([(cx+x-1,y),(cx+x+1,y)])
        line([(cx-10,35),(cx+10,35)])
    elif kind=='mic':
        box((cx-6,10,cx+6,30),6)
        draw.arc((int((cx-13)*S),20*S,int((cx+13)*S),38*S),0,180,fill='#ffffff',width=2*S)
        line([(cx,38),(cx,45)]);line([(cx-7,45),(cx+7,45)])
    else: box((cx-10,18,cx+10,38),3,fill='#ffffff')
    image.resize(size,Image.Resampling.LANCZOS).save(OUT/name)

input_icon('chat_keyboard_button.png',(260,56),'keyboard','#242a33')
input_icon('chat_keyboard_pressed.png',(260,56),'keyboard','#454f5f')
input_icon('chat_mic_button.png',(112,56),'mic','#353b47')
input_icon('chat_mic_pressed.png',(112,56),'mic','#454f5f')
input_icon('chat_stop_button.png',(112,56),'stop','#7f303b')
print('Created 5 opaque input icon assets')

compact_dir = OUT/'compact'
compact_dir.mkdir(exist_ok=True)
for path in OUT.glob('chat_*.png'):
    with Image.open(path) as original:
        original.resize((round(original.width*0.8),round(original.height*0.8)),Image.Resampling.LANCZOS).save(compact_dir/path.name)
print('Created icons at 80% scale')
