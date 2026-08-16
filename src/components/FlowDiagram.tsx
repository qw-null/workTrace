import { useEffect, useRef } from "react";

// 动态加载 mermaid，避免打进主包
let mermaidPromise: Promise<any> | null = null;
function loadMermaid(): Promise<any> {
  if (!mermaidPromise) {
    mermaidPromise = import("mermaid").then((mod) => {
      const m = mod.default;
      m.initialize({ startOnLoad: false, theme: "neutral" });
      return m;
    });
  }
  return mermaidPromise;
}

export default function FlowDiagram({ code }: { code: string }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    const id = "flow-" + Math.random().toString(36).slice(2, 8);
    loadMermaid()
      .then((m) => m.render(id, code))
      .then(({ svg }: { svg: string }) => {
        if (!cancelled && ref.current) ref.current.innerHTML = svg;
      })
      .catch(() => {
        if (!cancelled && ref.current) ref.current.textContent = code;
      });
    return () => {
      cancelled = true;
    };
  }, [code]);

  return <div ref={ref} className="flow-render" />;
}
