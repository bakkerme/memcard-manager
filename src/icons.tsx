import type { SVGProps } from "react";

type Props = SVGProps<SVGSVGElement>;

function Svg({ children, ...props }: Props) {
  return (
    <svg viewBox="0 0 24 24" width="1em" height="1em" aria-hidden focusable="false" {...props}>
      {children}
    </svg>
  );
}

const s = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.75,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

// PlayStation silhouette from Simple Icons (CC0), https://simpleicons.org/.
export function IconPlayStation(props: Props) {
  return (
    <Svg {...props}>
      <path fill="currentColor" d="M8.984 2.596v17.547l3.915 1.261V6.688c0-.69.304-1.151.794-.991.636.18.76.814.76 1.505v5.875c2.441 1.193 4.362-.002 4.362-3.152 0-3.237-1.126-4.675-4.438-5.827-1.307-.448-3.728-1.186-5.39-1.502zm4.656 16.241l6.296-2.275c.715-.258.826-.625.246-.818-.586-.192-1.637-.139-2.357.123l-4.205 1.5V14.98l.24-.085s1.201-.42 2.913-.615c1.696-.18 3.785.03 5.437.661 1.848.601 2.04 1.472 1.576 2.072-.465.6-1.622 1.036-1.622 1.036l-8.544 3.107V18.86zM1.807 18.6c-1.9-.545-2.214-1.668-1.352-2.32.801-.586 2.16-1.052 2.16-1.052l5.615-2.013v2.313L4.205 17c-.705.271-.825.632-.239.826.586.195 1.637.15 2.343-.12L8.247 17v2.074c-.12.03-.256.044-.39.073-1.939.331-3.996.196-6.038-.479z" />
    </Svg>
  );
}

export function IconFolder(props: Props) {
  return (
    <Svg {...props}>
      <path {...s} d="M3.6 19.2h16.8V8.7H12L10.3 6.8H3.6z" />
    </Svg>
  );
}

export function IconSearch(props: Props) {
  return <Svg {...props}><circle {...s} cx="10.5" cy="10.5" r="6" /><path {...s} d="m15 15 5 5" /></Svg>;
}

export function IconImport(props: Props) {
  return (
    <Svg {...props}>
      <path {...s} d="M12 4.6v10.2" />
      <path {...s} d="M8.2 11.6 12 15.4l3.8-3.8" />
      <path {...s} d="M5.2 19.4h13.6" />
    </Svg>
  );
}

export function IconBackup(props: Props) {
  return (
    <Svg {...props}>
      <path {...s} d="M12 15.4V5.2" />
      <path {...s} d="M8.2 8.4 12 4.6l3.8 3.8" />
      <path {...s} d="M5.2 19.4h13.6" />
    </Svg>
  );
}

export function IconPlus(props: Props) {
  return (
    <Svg {...props}>
      <path {...s} d="M12 5.2v13.6M5.2 12h13.6" />
    </Svg>
  );
}

export function IconCard(props: Props) {
  return (
    <Svg {...props}>
      <rect {...s} x="6.5" y="3.5" width="11" height="17" rx="2" />
      <rect {...s} x="8.7" y="6" width="6.6" height="4.6" rx="1" />
      <path {...s} d="M9.4 17.4v2.2M12 17.4v2.2M14.6 17.4v2.2" />
    </Svg>
  );
}

export function IconCopy(props: Props) {
  return (
    <Svg {...props}>
      <rect {...s} x="8.6" y="8.6" width="11" height="11" rx="2" />
      <path {...s} d="M15.4 8.6V6.2a1.8 1.8 0 0 0-1.8-1.8H6.2a1.8 1.8 0 0 0-1.8 1.8v7.4a1.8 1.8 0 0 0 1.8 1.8h2.4" />
    </Svg>
  );
}

export function IconStack(props: Props) {
  return (
    <Svg {...props}>
      <path {...s} d="M4.5 15.5 12 19.5l7.5-4" />
      <path {...s} d="M4.5 11.5 12 15.5l7.5-4" />
      <path {...s} d="M4.5 7.5 12 11.5l7.5-4L12 3.5z" />
    </Svg>
  );
}

export function IconArchive(props: Props) {
  return (
    <Svg {...props}>
      <path {...s} d="M4.4 8.4h15.2v10.8H4.4z" />
      <path {...s} d="M3.8 5.4h16.4v3H3.8z" />
      <path {...s} d="M10.2 13.4h3.6" />
    </Svg>
  );
}

export function IconCloud(props: Props) {
  return (
    <Svg {...props}>
      <path {...s} d="M7.4 17.4h9.3A3.7 3.7 0 0 0 18 10.3a5 5 0 0 0-9.6-1.1A3.6 3.6 0 0 0 7.4 17.4z" />
    </Svg>
  );
}

export function IconRefresh(props: Props) {
  return (
    <Svg {...props}>
      <path {...s} d="M19.6 12a7.6 7.6 0 1 1-2.2-5.4" />
      <path {...s} d="M19.6 5.4v5h-5" />
    </Svg>
  );
}

export function IconKebab(props: Props) {
  return (
    <Svg {...props}>
      <path
        fill="currentColor"
        d="M12 6.25a1.25 1.25 0 1 0 0-2.5 1.25 1.25 0 1 0 0 2.5zm0 7a1.25 1.25 0 1 0 0-2.5 1.25 1.25 0 1 0 0 2.5zm0 7a1.25 1.25 0 1 0 0-2.5 1.25 1.25 0 1 0 0 2.5z"
      />
    </Svg>
  );
}

export function IconPencil(props: Props) {
  return (
    <Svg {...props}>
      <path {...s} d="M14.2 5.3 18.7 9.8 9 19.5H4.5V15z" />
      <path {...s} d="M12.6 6.9 17.1 11.4" />
    </Svg>
  );
}

export function IconTrash(props: Props) {
  return (
    <Svg {...props}>
      <path {...s} d="M5 8.5h14M10 8.5V6.5h4v2M7.5 8.5l.8 11h7.4l.8-11" />
    </Svg>
  );
}

export function IconLink(props: Props) {
  return (
    <Svg {...props}>
      <path {...s} d="M10 13.5 8.5 15A3.2 3.2 0 1 1 4 10.5L6 8.5" />
      <path {...s} d="M14 10.5 15.5 9A3.2 3.2 0 1 1 20 13.5L18 15.5" />
    </Svg>
  );
}


export function IconClose(props: Props) {
  return <Svg {...props}><path {...s} d="m6 6 12 12M18 6 6 18" /></Svg>;
}

export function IconSettings(props: Props) {
  return <Svg {...props}>
    <path {...s} d="m9.5 3-.6 2.3-2 .9-2.2-.7-2.5 4.3 1.7 1.6v2.2l-1.7 1.6 2.5 4.3 2.2-.7 2 .9.6 2.3h5l.6-2.3 2-.9 2.2.7 2.5-4.3-1.7-1.6v-2.2l1.7-1.6-2.5-4.3-2.2.7-2-.9-.6-2.3z" />
    <circle {...s} cx="12" cy="12.5" r="3" />
  </Svg>;
}
