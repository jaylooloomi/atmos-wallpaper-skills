import io,json,sys,warnings
from PIL import Image
warnings.simplefilter('error',Image.DecompressionBombWarning)
Image.MAX_IMAGE_PIXELS=16_000_000
try:
 limit=(16 if sys.argv[1:]==['--normalized'] else 8)*1024*1024
 raw=sys.stdin.buffer.read(limit+1)
 if len(raw)>limit:raise ValueError('size')
 im=Image.open(io.BytesIO(raw))
 if im.format not in ('PNG','JPEG','WEBP'):raise ValueError('format')
 if getattr(im,'n_frames',1)!=1:raise ValueError('animated')
 if im.width<1 or im.height<1 or im.width>4096 or im.height>4096 or im.width*im.height>16_000_000:raise ValueError('dimensions')
 im.verify();im=Image.open(io.BytesIO(raw));im.load();im=im.convert('RGBA' if 'A' in im.getbands() else 'RGB');out=io.BytesIO();im.save(out,format='PNG');sys.stderr.write(json.dumps({'width':im.width,'height':im.height,'format':'PNG'}));sys.stdout.buffer.write(out.getvalue())
except Exception:
 sys.stderr.write('{"error":"Image rejected: unsupported, oversized, animated or undecodable raster"}');sys.exit(2)
