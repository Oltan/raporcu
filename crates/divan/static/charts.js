// Grafik ipuçları ve çizgi grafikte takip çizgisi. Değerler sayfada metin olarak da var;
// ipucu yalnızca kolaylık. Etiketler textContent ile yazılır (veri güvenilmez kabul edilir).
(function () {
  const tip = document.createElement("div");
  tip.className = "ipucu";
  tip.hidden = true;
  document.body.appendChild(tip);

  function show(lines, x, y) {
    tip.replaceChildren();
    lines.forEach((t, i) => {
      const e = document.createElement(i === 0 ? "b" : "div");
      if (i > 0) e.className = "ip-alt";
      e.textContent = t;
      tip.appendChild(e);
    });
    tip.hidden = false;
    const r = tip.getBoundingClientRect();
    let left = x + 14, top = y + 14;
    if (left + r.width > innerWidth - 8) left = x - r.width - 14;
    if (top + r.height > innerHeight - 8) top = y - r.height - 14;
    tip.style.left = left + "px";
    tip.style.top = top + "px";
  }
  function hide() { tip.hidden = true; }

  // Çubuk, dilim, sütun: işaretin kendisi hedef. "Ad: değer (%x)" → değer önde
  function split(s) {
    const i = s.lastIndexOf(": ");
    return i < 0 ? [s] : [s.slice(i + 2), s.slice(0, i)];
  }
  document.querySelectorAll("[data-tip]").forEach((el) => {
    el.addEventListener("pointermove", (e) => show(split(el.dataset.tip), e.clientX, e.clientY));
    el.addEventListener("pointerleave", hide);
    el.addEventListener("focus", () => { const r = el.getBoundingClientRect(); show(split(el.dataset.tip), r.right, r.top); });
    el.addEventListener("blur", hide);
  });

  // Çizgi grafik: en yakın koşuma oturan dikey takip çizgisi
  document.querySelectorAll("svg.cizgi-grafik[data-noktalar]").forEach((svg) => {
    const pts = JSON.parse(svg.dataset.noktalar || "[]");
    if (!pts.length) return;
    const line = svg.querySelector(".takip");
    const dots = svg.querySelectorAll(".nokta");
    const vb = svg.viewBox.baseVal;
    let current = -1;
    function nearest(e) {
      const r = svg.getBoundingClientRect();
      const x = vb.x + (e.clientX - r.left) * (vb.width / r.width);
      let best = 0;
      pts.forEach((p, i) => { if (Math.abs(p.x - x) < Math.abs(pts[best].x - x)) best = i; });
      return best;
    }
    svg.addEventListener("pointermove", (e) => {
      const i = nearest(e);
      const p = pts[i];
      line.setAttribute("x1", p.x); line.setAttribute("x2", p.x);
      line.setAttribute("y1", 12); line.setAttribute("y2", vb.height - 28);
      line.setAttribute("visibility", "visible");
      dots.forEach((d, k) => d.classList.toggle("aktif", k === i));
      current = i;
      show([p.deger + " geçti", p.baslik + " · " + p.ayrinti], e.clientX, e.clientY);
      svg.style.cursor = "pointer";
    });
    svg.addEventListener("pointerleave", () => { line.setAttribute("visibility", "hidden"); dots.forEach((d) => d.classList.remove("aktif")); hide(); current = -1; });
    svg.addEventListener("click", () => { if (current >= 0 && pts[current].href) location.href = pts[current].href; });
  });
})();
