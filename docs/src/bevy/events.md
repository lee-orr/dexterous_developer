# Replacing Events

Since events are ephemeral, there is no value in maintaining them as serializable elements like we do resources, components or states.

However, there can still be value in being able to register new events or replace old ones. To do so, simply use `app.add_event<E: Event>()` from within a reloadable scope. The usage here is identical to standard bevy events. Note that upon a reload, the event queue will be fully replaced - so you will miss any events issued during the previous frame.
