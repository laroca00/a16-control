import { useT, type Key } from "../i18n";
import { LiquidGlass } from "../ui/LiquidGlass";

export function ComingSoon({ title, body }: { title: Key; body: Key }) {
  const t = useT();
  return (
    <div className="page">
      <h1 className="page__title">{t(title)}</h1>
      <LiquidGlass className="soon" radius={24}>
        <h2 className="card__title">{t("soonTitle")}</h2>
        <p className="soon__body">{t(body)}</p>
      </LiquidGlass>
    </div>
  );
}
