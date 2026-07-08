# Reproduce the terminal demo as a GIF with charmbracelet/vhs:
#   vhs docs/demo.tape        # -> docs/demo.gif
#
# The committed docs/demo.svg is the hand-authored, dependency-free version used
# on the site and in the README. This tape is here so the same flow can be
# re-recorded from the real binary when you want a true-to-life GIF.

Output docs/demo.gif

Set Shell "bash"
Set FontFamily "JetBrains Mono"
Set FontSize 18
Set Width 1200
Set Height 640
Set Padding 24
Set Theme "Github Dark"
Set TypingSpeed 55ms

Hide
Type "export LOOM_HOME=$(mktemp -d)"  Enter
Type "clear"  Enter
Show

Type "loom search pdf"  Sleep 400ms  Enter
Sleep 2s

Type "loom install pdf"  Sleep 400ms  Enter
Sleep 2s

Type "loom list"  Sleep 400ms  Enter
Sleep 2500ms
