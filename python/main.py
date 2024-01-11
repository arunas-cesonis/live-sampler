import rtmidi
m = rtmidi.RtMidiOut()
for i in range(m.getPortCount()):
    print(i, m.getPortName(i))
#print(midiout.get_ports())
#port = mido.open_output('VirtualMidi Port1')
#port.send(msg)