"""Render captured native widget geometry for review without a device."""
from pathlib import Path
import json
from PIL import Image, ImageDraw, ImageFont
ROOT = Path(__file__).resolve().parent.parent
items = json.loads((ROOT/'.build/watch-layout.json').read_text(encoding='utf-8'))
im = Image.new('RGB', (432,514), '#0b0d10')
d = ImageDraw.Draw(im)
font = lambda size: ImageFont.truetype(str(ROOT/'assets/fonts/NotoSans-Regular.ttf'), size)
for item in items:
    p = item['config']; x,y,w,h = [p[k] for k in ('x','y','w','h')]
    if item['type'] == 'button':
        if 'normal_src' in p:
            asset = Image.open(ROOT/'vendor/penta-bip/assets/432x514-amazfit-bip-max'/p['normal_src']).convert('RGBA')
            im.paste(asset, (x,y), asset)
        else:
            d.rounded_rectangle((x,y,x+w-1,y+h-1),radius=p.get('radius',8),fill='#'+f"{p['normal_color']:06x}")
            d.text((x+w/2,y+h/2),p['text'],font=font(p['text_size']),fill='#ffffff',anchor='mm')
    elif item['type'] == 'text':
        d.text((x,y),p['text'],font=font(p['text_size']),fill='#'+f"{p['color']:06x}")
    elif item['type'] == 'list':
        surface=Image.new('RGB',(w,h),'#121418'); draw=ImageDraw.Draw(surface)
        config=p['item_config'][0];text=config['text_view'][0]
        for i,row in enumerate(p['data_array']):
            draw.text((text['x'],i*config['item_height']),row['line'],font=font(text['text_size']),fill='#ffffff')
        im.paste(surface,(x,y))
out=ROOT/'dist/PentaKuru-Watch-Chat-Preview.png';out.parent.mkdir(exist_ok=True)
im.save(out);print(out)
