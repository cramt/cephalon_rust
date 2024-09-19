# TODO

- determind amount of players as opposed to a 4 player hardcode
- create HashMap<HashSet<String>, Item> where the key is a set of the words in name of the item, then use that hashmaps keys to find the corrosponding item with the power of set intersections, this way we remove possible junk words
- retry in a few milisecs if we fail, best way to do this sorta "start a thread that we then stop later"-thing is to start a thread and give it a channel receiver, then send a oneshot channel receiver in the original channel, it then loops until the oneshot channel has a value and then it finishes, we then just send on the oneshot channel when we want it to stop
