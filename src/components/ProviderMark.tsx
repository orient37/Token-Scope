import { CircleHelp } from "lucide-react";
const claude = new URL("../assets/providers/claude.png", import.meta.url).href;
const deepseek = new URL("../assets/providers/deepseek.svg", import.meta.url)
  .href;
const glm = new URL("../assets/providers/glm.png", import.meta.url).href;
const kimiLight = new URL("../assets/providers/kimi-light.svg", import.meta.url)
  .href;
const kimiDark = new URL("../assets/providers/kimi-dark.svg", import.meta.url)
  .href;

const brands: Record<string, { name: string; src: string; darkSrc?: string }> =
  {
    "claude-code": { name: "Claude / Anthropic", src: claude },
    deepseek: { name: "DeepSeek", src: deepseek },
    glm: { name: "GLM / 智谱", src: glm },
    kimi: { name: "Kimi / Moonshot AI", src: kimiLight, darkSrc: kimiDark },
  };
// Source attribution, not a capability claim for unimplemented adapters.
export function ProviderMark({
  model,
  label = false,
  provider,
  size = 20,
}: {
  model?: string;
  label?: boolean;
  provider?: string;
  size?: number;
}) {
  const openai =
    provider !== undefined
      ? provider === "codex"
      : model === undefined || /^(gpt-|codex-)/i.test(model);
  const brand = provider === undefined ? undefined : brands[provider];
  const name = openai ? "OpenAI" : (brand?.name ?? "模型供应商未知");
  return (
    <span
      className={`provider-mark ${openai ? "openai" : brand ? provider : "unknown"}`}
      title={name}
    >
      {openai ? (
        <svg
          viewBox="0 0 32 32"
          width={size}
          height={size}
          role="img"
          aria-label="OpenAI 供应商标识"
        >
          <g fill="none" stroke="currentColor" strokeWidth="1.8">
            {[0, 60, 120, 180, 240, 300].map((a) => (
              <rect
                key={a}
                x="12"
                y="3"
                width="12"
                height="19"
                rx="6"
                transform={`rotate(${a} 16 16)`}
              />
            ))}
          </g>
        </svg>
      ) : brand ? (
        <>
          <img
            className={brand.darkSrc ? "provider-logo-light" : undefined}
            src={brand.src}
            width={size}
            height={size}
            alt={`${brand.name} 供应商标识`}
          />
          {brand.darkSrc && (
            <img
              className="provider-logo-dark"
              src={brand.darkSrc}
              width={size}
              height={size}
              alt={`${brand.name} 供应商标识`}
            />
          )}
        </>
      ) : (
        <CircleHelp size={size} aria-label="供应商未知" />
      )}
      {label && <span>{openai || brand ? name : "供应商未知"}</span>}
    </span>
  );
}
