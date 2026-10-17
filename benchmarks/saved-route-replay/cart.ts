export function subtotal(prices: number[]): number {
  if (prices.some((price) => !Number.isFinite(price) || price < 0)) {
    throw new RangeError("Prices must be finite and non-negative");
  }
  return prices.reduce((total, price) => total + price, 0);
}

export function totalWithTax(prices: number[], taxRate: number): number {
  if (!Number.isFinite(taxRate) || taxRate < 0) {
    throw new RangeError("Tax rate must be finite and non-negative");
  }
  return subtotal(prices) * (1 + taxRate);
}
