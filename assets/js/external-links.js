// This script adds ?ref=ssp.sh to all external links on your website
// Wait for window load to ensure all other scripts have had a chance to run
window.addEventListener('load', function() {
  // Get your website domain
  const siteDomain = window.location.hostname;
  const ownDomain = 'ssp.sh';
  
  // Find all links on the page
  const links = document.querySelectorAll('a');
  
  // Function to check if a hostname belongs to our domain
  // This checks only the domain part (hostname), not the path
  // For example, this will match ssp.sh, brain.ssp.sh, vault.ssp.sh, etc.
  // regardless of what path follows (like ssp.sh/brain/article)
  function isOwnDomain(hostname) {
    return hostname === ownDomain || 
           hostname.endsWith('.' + ownDomain) || 
           hostname === 'localhost' || 
           hostname.startsWith('localhost:');
  }
  
  // Process each link
  links.forEach(function(link) {
    // Check if the link has an href attribute
    if (link.href) {
      try {
        // Get the URL of the link
        const linkUrl = new URL(link.href);

        // Only process http(s) URLs — skip javascript:, mailto:, tel:, etc.
        if (linkUrl.protocol !== 'http:' && linkUrl.protocol !== 'https:') {
          return;
        }

        // Check if the link is external (different hostname) and not to our own domains
        if (linkUrl.hostname !== siteDomain && !isOwnDomain(linkUrl.hostname)) {
          // Only modify if the URL doesn't already have the ref parameter
          if (!linkUrl.searchParams.has('ref')) {
            // Store the hash part (including text fragments)
            const hashPart = linkUrl.hash;
            
            // Remove the hash part temporarily
            linkUrl.hash = '';
            
            // Add the ref parameter
            linkUrl.searchParams.set('ref', 'ssp.sh');
            
            // Re-add the hash part
            const urlWithRef = linkUrl.toString() + hashPart;
            
            // Update the link's href attribute
            link.href = urlWithRef;
          }
        }
      } catch (e) {
        // Ignore invalid URLs
        console.warn('Invalid URL found:', link.href);
      }
    }
  });

  // Add click event listener for dynamically added links
  document.body.addEventListener('click', function(event) {
    // Check if the clicked element is an anchor tag or contains one
    const link = event.target.closest('a');
    if (link && link.href) {
      try {
        const linkUrl = new URL(link.href);

        // Only process http(s) URLs — skip javascript:, mailto:, tel:, etc.
        if (linkUrl.protocol !== 'http:' && linkUrl.protocol !== 'https:') {
          return;
        }

        // Only process external links that aren't to our own domains
        if (linkUrl.hostname !== siteDomain && !isOwnDomain(linkUrl.hostname) && !linkUrl.searchParams.has('ref')) {
          // Prevent the default action
          event.preventDefault();
          
          // Store the hash part (including text fragments)
          const hashPart = linkUrl.hash;
          
          // Remove the hash part temporarily
          linkUrl.hash = '';
          
          // Add the ref parameter
          linkUrl.searchParams.set('ref', 'ssp.sh');
          
          // Re-add the hash part
          const urlWithRef = linkUrl.toString() + hashPart;
          
          // Navigate to the modified URL
          window.open(urlWithRef, link.target || '_self');
        }
      } catch (e) {
        // Let the browser handle invalid URLs
      }
    }
  }, false);
});
