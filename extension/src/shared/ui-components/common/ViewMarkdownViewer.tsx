import { styled } from "@mui/material";
import { FC } from "react";
import Markdown from "react-markdown";

type ViewMarkdownViewerProps = {
  content: string | undefined;
};

export const ViewMarkdownViewer: FC<ViewMarkdownViewerProps> = ({ content }) => {
  return (
    <StyledMarkdown skipHtml components={{ a: LinkRenderer }}>
      {content}
    </StyledMarkdown>
  );
};

const StyledMarkdown = styled(Markdown)(({ theme }) => ({
  fontSize: theme.typography.body2.fontSize,
  lineHeight: 1.6,
  background: "transparent",

  h1: {
    fontSize: "2.25em",
    fontWeight: 700,
    marginTop: "1em",
    marginBottom: "0.5em",
  },
  h2: {
    fontSize: "1.875em",
    fontWeight: 600,
    marginTop: "1.25em",
    marginBottom: "0.5em",
  },
  h3: {
    fontSize: "1.5em",
    fontWeight: 600,
    marginTop: "1em",
    marginBottom: "0.4em",
  },
  h4: {
    fontSize: "1.25em",
    fontWeight: 600,
    marginTop: "1em",
    marginBottom: "0.4em",
  },
  h5: {
    fontSize: "1em",
    fontWeight: 600,
    marginTop: "0.8em",
    marginBottom: "0.4em",
  },
  h6: {
    fontSize: "0.875em",
    fontWeight: 600,
    textTransform: "uppercase",
    marginTop: "0.8em",
    marginBottom: "0.4em",
  },

  p: {
    marginBottom: "1em",
  },

  blockquote: {
    borderLeft: "4px solid currentColor",
    paddingLeft: "1em",
    fontStyle: "italic",
    margin: "1.5em 0",
  },

  ul: {
    listStyleType: "disc",
    marginLeft: "1.5em",
    marginBottom: "1em",
  },

  ol: {
    listStyleType: "decimal",
    marginLeft: "1.5em",
    marginBottom: "1em",
  },

  li: {
    marginBottom: "0.5em",
  },

  code: {
    padding: "0.2em 0.4em",
    borderRadius: "0.25em",
    fontSize: "0.95em",
  },

  pre: {
    fontSize: "0.95em",
    padding: "1em",
    borderRadius: "0.375em",
    overflowX: "auto",
    margin: "1.5em 0",
  },

  "pre code": {
    background: "none",
    padding: 0,
  },

  table: {
    width: "100%",
    borderCollapse: "collapse",
    margin: "2em 0",
  },

  th: {
    border: "1px solid currentColor",
    fontWeight: 600,
    padding: "0.75em",
    textAlign: "left",
  },

  td: {
    border: "1px solid currentColor",
    padding: "0.75em",
    textAlign: "left",
  },

  hr: {
    border: "none",
    borderTop: "1px solid currentColor",
    margin: "2em 0",
  },

  img: {
    maxWidth: "100%",
    height: "auto",
    borderRadius: "0.5em",
    margin: "1em 0",
    display: "block",
  },

  video: {
    maxWidth: "100%",
    height: "auto",
    borderRadius: "0.5em",
    margin: "1em 0",
    display: "block",
  },

  a: {
    textDecoration: "underline",
  },
}));

function LinkRenderer(props: any) {
  return (
    <a href={props.href} target="_blank" rel="noreferrer">
      {props.children}
    </a>
  );
}
