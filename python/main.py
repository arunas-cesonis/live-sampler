import mido
msg = mido.Message('note_on', note=60)
port = mido.open_output('VirtualMidi Port1')
port.send(msg)