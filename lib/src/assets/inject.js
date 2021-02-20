// This code was inserted by the 'penguin' library. It's here to enable features
// like browser auto-reloading or showing messages. It does this by
// communicating with the penguin server via a websocket.

// Configuration dependent values that are passed/interpolated by the penguin
// server.
const control_path = "{{ control_path }}";


// The target URI of the websocket connection.
const wsUri = (() => {
    const scheme = window.location.protocol === "https" ? "wss" : "ws";
    const host = window.location.host;
    return `${scheme}://${host}${control_path}`;
})();


function onConnectionError() {
    console.warn(`Could not connect to web socket backend ${wsUri}`);
}

// Open websocket connection and install handlers.
const socket = new WebSocket(wsUri);
socket.addEventListener("close", onConnectionError);
socket.addEventListener("open", () => {
    socket.removeEventListener("close", onConnectionError)
    socket.addEventListener("close", () => console.log("penguin server closed WS connection"));
});
