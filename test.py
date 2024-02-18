def process(buf, events):
  nc = len(buf)
  if nc == 0:
    return buf
  ns = len(buf[0])
  if ns == 0:
    return buf
  for i in range(ns):
    for j in range(nc):
      buf[j][i] = buf[j][i] * 0.5
  return (buf, events)
