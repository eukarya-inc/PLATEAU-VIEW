import { ReactNode } from "react";

export const OptionSection = ({ title, children }: { title: string; children: ReactNode }) => (
  <div className="flex flex-col gap-2">
    <h3 className="text-sm font-semibold text-purple-800 uppercase text-foreground">{title}</h3>
    {children}
  </div>
);
