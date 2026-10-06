const badge = document.createElement("div");
badge.textContent = "slix ✓";
Object.assign(badge.style, {
  position: "fixed",
  bottom: "16px",
  left: "16px",
  zIndex: "9999",
  padding: "4px 10px",
  borderRadius: "999px",
  background: "#1d9bf0",
  color: "white",
  font: "bold 13px system-ui, sans-serif",
  pointerEvents: "none",
});
document.body.appendChild(badge);

console.log(`slix: running on ${location.hostname}`);
