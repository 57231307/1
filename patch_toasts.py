import os
import re
import glob

# We will remove `use gloo_dialogs;` and replace `gloo_dialogs::alert` with something else.
# But since we are in Yew `Component` lifecycle (not function components), we can't easily use `use_reducer` hook everywhere.
# We can use a global EventBus for toasts in frontend, or just a simple `web_sys::window().unwrap().alert_with_message` 
# Wait, `gloo_dialogs::alert` is basically `window.alert`. It's blocking.
# The audit asks to replace it with Toast.
# Let's create a global static for Toast in frontend using a channel, or simpler: since it's a SPA, we can mount a global Toast component and send messages to it.
pass
