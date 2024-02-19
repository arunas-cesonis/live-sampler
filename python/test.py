def process(state, buf, events):
  nc = len(buf)
  if nc == 0:
    return buf
  ns = len(buf[0])
  if ns == 0:
    return buf
  for i in range(ns):
    for j in range(nc):
      buf[j][i] = buf[j][i] * 0.5
  if not isinstance(state, dict):
    state = {"counter": 0}
  state["counter"] += 5
  x = []
  for e in events:
    e.note = 21
    e.velocity= 0.66
    x.append(host.NoteOn(e.note, e.channel, e.timing, e.velocity, e.voice_id))
  if x:
    host.print((x, state))
  return (state, buf, events)
