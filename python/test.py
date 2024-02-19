def process(state, buf, events):
  if state is None:
    state = {"prev": [0] * len(buf)}
  for j in range(len(buf)):
    prev = state["prev"][j]
    
    for i in range(1, len(buf[j])):
      cur = buf[j][i]
      buf[j][i] = (prev + cur) * 0.5
      prev = cur
    state["prev"][j] = prev

  return (state, buf, events)

