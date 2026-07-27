component Cover(source:str, size:f64=42.0, radius:f64=9.0)
  box #root w=size h=size clip=true r=radius shadow=black/16 shadow-y=3.0 shadow-blur=8.0
    image source #image w=size h=size fit=cover r=radius
