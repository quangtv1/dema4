import { ApplicationConfig, provideBrowserGlobalErrorListeners } from '@angular/core';

// Không dùng router: màn hiện tại là signal `UiService.screen` (prototype đổi màn bằng `S.screen`).
export const appConfig: ApplicationConfig = {
  providers: [provideBrowserGlobalErrorListeners()],
};
