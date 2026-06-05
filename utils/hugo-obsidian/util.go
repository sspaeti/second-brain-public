package main

import (
	"fmt"
	"net/url"
	"path/filepath"
	"regexp"
	"strings"
	"unicode"
)

func trim(source, prefix, suffix string) string {
	return strings.TrimPrefix(strings.TrimSuffix(source, suffix), prefix)
}

var indexSuffixRe = regexp.MustCompile(`/index(\.\w{2})?$`)

func hugoPathTrim(source string) string {
	// Strip /index, /index.en, /index.de, etc. and _index variants
	source = indexSuffixRe.ReplaceAllString(source, "")
	return strings.TrimSuffix(source, "_index")
}

func processTarget(source string) string {
	if !isInternal(source) {
		return source
	}
	if strings.HasPrefix(source, "/") {
		return strings.TrimSuffix(source, ".md")
	}
	// Handle case sensitivity and anchor fragments
	res := strings.Split(source, "#")[0]
	// Convert to lowercase for case-insensitive matching
	res = strings.ToLower(res)
	res = "/" + strings.TrimSuffix(strings.TrimSuffix(res, ".html"), ".md")
	res, _ = url.PathUnescape(res)
	res = strings.TrimSpace(res)
	
	// Special handling for title slashes that might be in links
	res = strings.ReplaceAll(res, " / ", "-")
	res = UnicodeSanitize(res)
	return strings.ReplaceAll(url.PathEscape(res), "%2F", "/")
}

func processSource(source string) string {
	res := filepath.ToSlash(hugoPathTrim(source))
	// Convert to lowercase for consistent matching
	res = strings.ToLower(res)
	// Special handling for title slashes that might be in links
	res = strings.ReplaceAll(res, " / ", "-")
	res = UnicodeSanitize(res)
	return strings.ReplaceAll(url.PathEscape(res), "%2F", "/")
}

func isInternal(link string) bool {
	return !strings.HasPrefix(link, "http")
}

// From https://golang.org/src/net/url/url.go
func ishex(c rune) bool {
	switch {
	case '0' <= c && c <= '9':
		return true
	case 'a' <= c && c <= 'f':
		return true
	case 'A' <= c && c <= 'F':
		return true
	}
	return false
}

// UnicodeSanitize sanitizes string to be used in Hugo URL's
// from https://github.com/gohugoio/hugo/blob/93aad3c543828efca2adeb7f96cf50ae29878593/helpers/path.go#L94
//
// MIRRORED IN: ../obsidian-quartz/src/enrich_with_blog.rs::unicode_sanitize
// (keep the two implementations in sync — the Rust copy is used to match
// brain canonical slugs when enriching brain↔blog cross-site links).
func UnicodeSanitize(s string) string {
	source := []rune(s)
	target := make([]rune, 0, len(source))
	var prependHyphen bool

	for i, r := range source {
		isAllowed := r == '.' || r == '/' || r == '\\' || r == '_' || r == '#' || r == '+' || r == '~'
		isAllowed = isAllowed || unicode.IsLetter(r) || unicode.IsDigit(r) || unicode.IsMark(r)
		isAllowed = isAllowed || (r == '%' && i+2 < len(source) && ishex(source[i+1]) && ishex(source[i+2]))

		if isAllowed {
			if prependHyphen {
				target = append(target, '-')
				prependHyphen = false
			}
			target = append(target, r)
		} else if len(target) > 0 && (r == '-' || unicode.IsSpace(r)) {
			prependHyphen = true
		}
	}

	return string(target)
}

// filter out certain links (e.g. to media)
func filter(links []Link) (res []Link) {
	for _, l := range links {
		// filter external and non-md
		isMarkdown := filepath.Ext(l.Target) == "" || filepath.Ext(l.Target) == ".md"
		if isInternal(l.Target) && isMarkdown {
			res = append(res, l)
		}
	}
	fmt.Printf("Removed %d external and non-markdown links\n", len(links)-len(res))
	return res
}
