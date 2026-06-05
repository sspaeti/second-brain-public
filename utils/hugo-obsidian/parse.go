package main

import (
	"bytes"
	"fmt"
	"github.com/PuerkitoBio/goquery"
	"io/ioutil"
	"strings"
)

// parse single file for links
func parse(dir, pathPrefix string) []Link {
	// read file
	source, err := ioutil.ReadFile(dir)
	if err != nil {
		panic(err)
	}

	// parse md
	var links []Link
	fmt.Printf("[Parsing note] %s => ", trim(dir, pathPrefix, ".md"))

	var buf bytes.Buffer
	if err := md.Convert(source, &buf); err != nil {
		panic(err)
	}

	doc, err := goquery.NewDocumentFromReader(&buf)
	var n int
	doc.Find("a").Each(func(i int, s *goquery.Selection) {
		text := strings.TrimSpace(s.Text())
		target, ok := s.Attr("href")
		if !ok {
			target = "#"
		}

		// Extract the base target without any block references
		baseTarget := target
		if blockRefIndex := strings.Index(target, "^"); blockRefIndex != -1 {
			baseTarget = target[:blockRefIndex]
		}

		target = processTarget(baseTarget)
		source := processSource(trim(dir, pathPrefix, ".md"))

		// Don't skip links with block references in the text
		// Only skip ones that are just a block reference (^...)
		if !strings.HasPrefix(text, "^") || strings.Contains(text, " ") {
			links = append(links, Link{
				Source: source,
				Target: target,
				Text:   text,
			})
			n++
		}
	})
	fmt.Printf("found: %d links\n", n)

	return links
}
