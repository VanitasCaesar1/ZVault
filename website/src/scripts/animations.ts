import { gsap } from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

gsap.registerPlugin(ScrollTrigger);

// Animate all [data-animate] elements on scroll
document.querySelectorAll('[data-animate]').forEach((el) => {
  gsap.from(el, {
    y: 30,
    opacity: 0,
    duration: 0.8,
    ease: 'power3.out',
    scrollTrigger: {
      trigger: el,
      start: 'top 85%',
      once: true,
    },
  });
});

// Stagger feature cards
const featureCards = document.querySelectorAll('#features [data-animate]');
if (featureCards.length) {
  gsap.from(featureCards, {
    y: 40,
    opacity: 0,
    duration: 0.6,
    stagger: 0.08,
    ease: 'power3.out',
    scrollTrigger: {
      trigger: '#features',
      start: 'top 75%',
      once: true,
    },
  });
}

// Comparison table rows slide in
const tableRows = document.querySelectorAll('#comparison tbody tr');
if (tableRows.length) {
  gsap.from(tableRows, {
    x: -20,
    opacity: 0,
    duration: 0.5,
    stagger: 0.06,
    ease: 'power2.out',
    scrollTrigger: {
      trigger: '#comparison table',
      start: 'top 80%',
      once: true,
    },
  });
}

// Pricing cards pop in
const pricingCards = document.querySelectorAll('#pricing .grid > div');
if (pricingCards.length) {
  gsap.from(pricingCards, {
    y: 30,
    opacity: 0,
    duration: 0.6,
    stagger: 0.1,
    ease: 'back.out(1.2)',
    scrollTrigger: {
      trigger: '#pricing',
      start: 'top 75%',
      once: true,
    },
  });
}

// Nav background on scroll
const nav = document.getElementById('nav');
if (nav) {
  ScrollTrigger.create({
    start: 'top -80',
    onUpdate: (self) => {
      if (self.direction === 1 && self.scroll() > 80) {
        nav.style.borderBottom = '1px solid rgba(39, 39, 42, 0.5)';
      } else if (self.scroll() <= 80) {
        nav.style.borderBottom = 'none';
      }
    },
  });
}
