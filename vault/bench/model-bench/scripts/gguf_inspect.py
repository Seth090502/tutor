#!/usr/bin/env python
"""Print GGUF metadata and tensor names (cls/output) from a local file or a Hugging Face resolve URL (HTTP range read of the header).

  python gguf_inspect.py <path-or-url> ...
Used to confirm a reranker conversion carries pooling_type=4 (rank), classifier.output_labels, tokenizer.chat_template.rerank and a
cls.output.weight tensor before trusting it.
"""
import json,struct,urllib.request as u,io,sys
def hf(url,rng=None):
    h={'User-Agent':'x'}
    if rng: h['Range']='bytes=%d-%d'%rng
    return u.urlopen(u.Request(url,headers=h),timeout=120)
class R:
    def __init__(s,b): s.b=b;s.i=0
    def rd(s,n):
        if s.i+n>len(s.b): raise EOFError
        v=s.b[s.i:s.i+n]; s.i+=n; return v
    def unpack(s,f): return struct.unpack('<'+f,s.rd(struct.calcsize(f)))[0]
    def string(s): n=s.unpack('Q'); return s.rd(n).decode('utf-8','replace')
SZ={0:'B',1:'b',2:'H',3:'h',4:'I',5:'i',6:'f',7:'?',10:'Q',11:'q',12:'d'}
def val(r,t):
    if t in SZ: return r.unpack(SZ[t])
    if t==8: return r.string()
    if t==9:
        et=r.unpack('I'); n=r.unpack('Q')
        if et==8:
            return [r.string() for _ in range(n)] if n<64 else ('<str array %d>'%n, [r.string() for _ in range(n)][:0])[0]
        if et in SZ:
            sz=struct.calcsize(SZ[et]); r.rd(sz*n); return '<arr %d>'%n
        raise ValueError(et)
    raise ValueError(t)
def parse(url,nbytes=24<<20):
    b=open(url,'rb').read(nbytes) if not url.startswith('http') else hf(url,(0,nbytes-1)).read()
    r=R(b)
    assert r.rd(4)==b'GGUF'
    ver=r.unpack('I'); nt=r.unpack('Q'); nk=r.unpack('Q')
    kv={}
    for _ in range(nk):
        k=r.string(); t=r.unpack('I'); kv[k]=val(r,t)
    tens=[]
    for _ in range(nt):
        n=r.string(); nd=r.unpack('I'); dims=[r.unpack('Q') for _ in range(nd)]; tp=r.unpack('I'); off=r.unpack('Q'); tens.append((n,dims,tp))
    return ver,kv,tens
if __name__=='__main__':
    for url in sys.argv[1:]:
        try:
            ver,kv,tens=parse(url)
            print('==',url)
            for k,v in kv.items():
                if k.startswith('tokenizer.ggml') and k not in('tokenizer.ggml.model','tokenizer.ggml.eos_token_id','tokenizer.ggml.bos_token_id','tokenizer.ggml.padding_token_id'): continue
                s=str(v); print('  ',k,'=',s[:300].replace('\n','\n'))
            print('   cls tensors:',[t for t in tens if 'cls' in t[0] or 'output' in t[0] or 'cls_out' in t[0]])
        except Exception as e: print('==',url,'ERR',repr(e))
